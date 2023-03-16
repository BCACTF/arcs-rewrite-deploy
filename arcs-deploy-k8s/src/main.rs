use futures::{StreamExt, TryStreamExt};
use kube::{Client, api::{Api, ResourceExt, ListParams, PostParams}};
use k8s_openapi::api::core::v1::Pod;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};

use arcs_deploy_k8s::{logging, create_client, get_pods, create_challenge, delete_challenge};
// make sure to update k8s version used in Cargo.toml

use std::io::{Result as IOResult, Error as IOError};

use dotenv::dotenv;
extern crate dotenv;

#[tokio::main]
async fn main() -> IOResult<()>{
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;

    dotenv().ok();

    let client: Client = match create_client().await {
        Ok(client) => client,
        Err(e) => {
            return Err(IOError::new(std::io::ErrorKind::Other, e));
        }
    };


    // delete_challenge(client.clone(), vec!["real-deal-html"]).await.unwrap();
    // create_challenge(client.clone(), vec!["bof-shop"], "/Users/yusuf/Documents/code/arcs/arcs-rewrite/testdockerdirectory").await;
    // create_challenge(client.clone(), vec!["real-deal-html"], "/Users/yusuf/Documents/code/arcs/arcs-rewrite/testdockerdirectory").await;
    // generate_registry_secret(client.clone()).await;

    Ok(())
}
