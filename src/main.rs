use actix_web::{
    post, put,
    web::Data as WebData,
    App, HttpResponse, HttpServer, Responder,
};
use tokio_postgres::Client;

use std::{
    io::Result as IOResult,
    sync::Arc,
};

use deploy::database::database_init;
use arcs_deploy_logging::set_up_logging;

/// TODO: Use the inner properties so we can remove the `#[allow(unused)]` annotation
#[allow(unused)]
#[derive(Debug, Clone)]
struct AppState {
    db_client: Arc<Client>,
}


#[actix_web::main]
async fn main() -> IOResult<()> {
    set_up_logging(&arcs_deploy_logging::DEFAULT_LOGGGING_TARGETS, deploy::logging::DEFAULT_TARGET_NAME)?;

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
