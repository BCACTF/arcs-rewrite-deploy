use arcs_logging_rs::{set_up_logging, DEFAULT_LOGGGING_TARGETS};

use arcs_deploy_k8s::logging;
// make sure to update k8s version used in Cargo.toml

use std::io::Result as IOResult;

use dotenv::dotenv;
extern crate dotenv;

#[tokio::main]
async fn main() -> IOResult<()>{
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    dotenv().ok();

    check_env_vars().expect("Missing environment variables");

    Ok(())
}
