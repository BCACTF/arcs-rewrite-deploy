use arcs_deploy_main::env::check_env_vars;
use arcs_deploy_main::start_server;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_main::logging;

use std::io::{Result as IOResult};

use dotenv::dotenv;

#[tokio::main]
async fn main() -> IOResult<()> {

    dotenv().ok(); // load env vars

    check_env_vars().expect("Missing environment variables");

    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    start_server().await;
    Ok(())
}