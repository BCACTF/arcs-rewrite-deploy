mod server;

pub use crate::server::emitter;
pub use crate::server::receiver;
use server::initialize_server;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub async fn start_server() {
    info!("Initializing webhook server...");
    match initialize_server().await {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to start Deploy server");
            error!("Trace: {}", e);
        },
    };
}