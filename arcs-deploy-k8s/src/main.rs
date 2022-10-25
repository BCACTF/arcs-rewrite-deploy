use futures::{StreamExt, TryStreamExt};
use kube::{Client, api::{Api, ResourceExt, ListParams, PostParams}};
use k8s_openapi::api::core::v1::Pod;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};

use arcs_deploy_k8s::{logging, create_client, get_pods, create_challenge, delete_challenge, generate_registry_secret};
// make sure to update k8s version used in Cargo.toml

use dotenv::dotenv;
extern crate dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;

    dotenv().ok();

    let client: Client = create_client().await.unwrap();
    delete_challenge(client.clone(), vec!["real-deal-html"]).await.unwrap();
    // create_challenge(client.clone(), vec!["real-deal-html"], "/Users/yusuf/documents/code/bcactf3.0/bcactf-3.0/").await;

    generate_registry_secret(client.clone()).await;

    Ok(())
}
