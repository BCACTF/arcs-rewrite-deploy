use arcs_deploy_webhook::start_server;

#[tokio::main]
async fn main() {
    start_server().await;
}