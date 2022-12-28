mod server;

pub use crate::server::emitter;
pub use crate::server::receiver;
use server::initialize_server;

pub async fn start_server() {
    initialize_server().await;
}