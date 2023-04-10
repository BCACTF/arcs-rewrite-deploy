use reqwest::Client;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

mod env;

use env::*;

pub async fn deploy_static_chall(name: &str) {
    info!("Deploying static challenge: {}", name);
    let client = Client::new();
    
    let url = format!("{}/{}", s3_bucket_url(), name);
    let res = client.get(&url).bearer_auth(s3_bearer_token()).send().await;
    match res {
        Ok(_) => info!("Successfully deployed static challenge: {}", name),
        Err(e) => error!("Failed to deploy static challenge: {}", e),
    }

}