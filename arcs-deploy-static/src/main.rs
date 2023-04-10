use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_static::logging;

use dotenv::dotenv;

use arcs_deploy_static::env::check_env_vars;

#[tokio::main]
async fn main() -> IOResult<()> {
    dotenv().ok();
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    check_env_vars().expect("Missing environment variables");
    arcs_deploy_static::deploy_static_chall("test");
}
