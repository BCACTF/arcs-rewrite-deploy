mod server;

pub use crate::server::emitter;
pub use crate::server::receiver;
use server::initialize_server;

// TODO --> improve env var system
use dotenv::dotenv;

pub async fn start_server() {
    dotenv().ok(); // load env vars

    match initialize_server().await {
        Ok(_) => {},
        Err(_) => {},
    };
}