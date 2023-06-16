use arcs_logging_rs::{set_up_logging, DEFAULT_LOGGGING_TARGETS};

use std::io::Result as IOResult;

use arcs_deploy_docker::check_env_vars;

use arcs_deploy_docker::logging;

extern crate dotenv;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> IOResult<()> {
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    dotenv().ok();

    check_env_vars().expect("Missing environment variables");

    Ok(())
}