use futures::{StreamExt, TryStreamExt};
use kube::{Client, api::{Api, ResourceExt, ListParams, PostParams}};
use k8s_openapi::api::core::v1::Pod;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};

use arcs_deploy_k8s::{logging, create_client, get_pods, create_challenge, delete_challenge};
// make sure to update k8s version used in Cargo.toml

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;
    let client: Client = create_client().await;

    

    Ok(())
}
