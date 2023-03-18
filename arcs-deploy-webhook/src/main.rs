use arcs_deploy_webhook::start_server;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_webhook::logging;

use std::io::{Result as IOResult};

// TODO --> improve env var system
use dotenv::dotenv;

#[tokio::main]
async fn main() -> IOResult<()> {

    dotenv().ok(); // load env vars
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    start_server().await;
    Ok(())
}