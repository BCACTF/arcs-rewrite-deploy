use arcs_deploy_webhook::start_server;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_webhook::logging;

use std::io::{Result as IOResult};
use std::env;

use dotenv::dotenv;

#[tokio::main]
async fn main() -> IOResult<()> {

    dotenv().ok(); // load env vars

    env::var("DEPLOY_SERVER_AUTH_TOKEN").expect("DEPLOY_SERVER_AUTH_TOKEN must be set");
    env::var("WEBHOOK_SERVER_AUTH_TOKEN").expect("WEBHOOK_SERVER_AUTH_TOKEN must be set");

    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    start_server().await;
    Ok(())
}