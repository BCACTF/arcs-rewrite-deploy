use arcs_docker::check_env_vars as check_docker_env_vars;
use arcs_k8s::check_env_vars as check_k8s_env_vars;
use arcs_static::env::check_env_vars as check_static_env_vars;
use arcs_deploy_main::env::check_env_vars;
use arcs_deploy_main::start_server;

use arcs_logging_rs::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_main::logging;

use std::io::Result as IOResult;

use dotenvy::dotenv;

#[tokio::main]
async fn main() -> IOResult<()> {
    dotenv().ok(); // load env vars

    // Ensure all required env vars are set
    check_env_vars().expect("Missing environment variables");
    check_docker_env_vars().expect("Missing docker environment variables");
    check_k8s_env_vars().expect("Missing k8s environment variables");
    check_static_env_vars().expect("Missing static environment variables");

    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    start_server().await;
    Ok(())
}