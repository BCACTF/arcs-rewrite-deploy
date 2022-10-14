use std::sync::Arc;
use std::io::Result as IOResult;


// Server imports
use deploy::webhooks::*;

// Logging imports
use arcs_deploy_logging::set_up_logging;

// Database imports
use tokio_postgres::Client;
use deploy::database::database_init;



/// TODO: Use the inner properties so we can remove the `#[allow(unused)]` annotation
#[allow(unused)]
#[derive(Debug, Clone)]
struct AppState {
    db_client: Arc<Client>,
}


#[actix_web::main]
async fn main() -> IOResult<()> {
    use actix_web::App as ActixApp;
    use actix_web::web::Data as AppData;
    use actix_web::HttpServer as ActixHttpServer;


    set_up_logging(&arcs_deploy_logging::DEFAULT_LOGGGING_TARGETS, deploy::logging::DEFAULT_TARGET_NAME)?;

    let postgres_client = database_init().await?;
    let postgres_client_arc = AppState {
        db_client: Arc::new(postgres_client),
    };

    ActixHttpServer::new(move || {
        ActixApp::new()
            .app_data(AppData::new(postgres_client_arc.clone()))
            .service(github::github_webhook)
            // .service(update_keys)
    })
        .bind(("0.0.0.0", 8085))?
        .run()
        .await
}

// #[put("/update_keys")]
// async fn update_keys() -> impl ActixResponder {
//     use actix_web::HttpResponse;

//     HttpResponse::Ok().body("")
// }

