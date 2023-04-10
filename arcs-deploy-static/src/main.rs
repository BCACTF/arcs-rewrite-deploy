use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_static::logging;

use dotenv::dotenv;

use std::io::Result as IOResult;

use arcs_deploy_static::env::check_env_vars;

use arcs_deploy_static::deploy_static_files;

#[tokio::main]
async fn main() -> IOResult<()> {
    dotenv().ok();
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    check_env_vars().expect("Missing environment variables");
    match deploy_static_files("ehrenfest").await {
        Ok(_) => println!("Success"),
        Err(e) => println!("Failure {:#?}", e),
    };

    Ok(())
}
