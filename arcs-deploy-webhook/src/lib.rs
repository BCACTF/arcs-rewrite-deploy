mod server;

pub use crate::server::emitter;
pub use crate::server::receiver;
use server::initialize_server;

// TODO --> improve env var system
use dotenv::dotenv;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub async fn start_server() {
    dotenv().ok(); // load env vars
    info!("Initializing webhook server...");
    match initialize_server().await {
        Ok(_) => {},
        Err(_) => {},
    };
}