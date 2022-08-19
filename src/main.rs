use actix_web::{
    post, put,
    web::Data as WebData,
    App, HttpResponse, HttpServer, Responder,
};
use lazy_static::lazy_static;
use tokio_postgres::Client;

use std::{
    io::Result as IOResult,
    sync::Arc,
    path::Path,
};


use smallvec::smallvec;


use deploy::database::database_init;
use arcs_deploy_logging::{set_up_logging, LogLocationTargetMap};

#[derive(Debug, Clone)]
struct AppState {
    db_client: Arc<Client>,
}

lazy_static! {
    

    static ref ERR_FILE: &'static Path = Path::new("./err.log");
    static ref ERR_WARN_FILE: &'static Path = Path::new("./err_warn.log");
    static ref INFO_DEBUG_FILE: &'static Path = Path::new("./info_debug.log");

    
    static ref DEFAULT_LOGGGING_TARGET: LogLocationTargetMap<'static> = {
        use arcs_deploy_logging::Level::*;
        use arcs_deploy_logging::LogLocationTarget::*;
        vec![
            (Trace, smallvec![
                StdOut,
            ]),
            (Debug, smallvec![
                StdOut,
                File(&INFO_DEBUG_FILE),
            ]),
            (Info, smallvec![
                StdOut,
                File(&INFO_DEBUG_FILE),
            ]),
            (Warn, smallvec![
                StdErr,
                File(&ERR_WARN_FILE),
            ]),
            (Error, smallvec![
                StdErr,
                File(&ERR_FILE),
                File(&ERR_WARN_FILE),
            ]),
            
        ].into_iter().collect()
    };
}


#[actix_web::main]
async fn main() -> IOResult<()> {
    set_up_logging(&DEFAULT_LOGGGING_TARGET)?;

    let postgres_client = database_init().await?;
    let postgres_client_arc = AppState {
        db_client: Arc::new(postgres_client),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(WebData::new(postgres_client_arc.clone()))
            .service(github_webhook)
            .service(update_keys)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[put("/update_keys")]
async fn update_keys() -> impl Responder {
    HttpResponse::Ok().body("")
}

#[post("/github_webhook")]
async fn github_webhook(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}
