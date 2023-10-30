mod deployment_failure_req;
mod deployment_success_req;
mod meta;
mod sync;

use reqwest::Client;

use arcs_yaml_parser::deploy::structs::DeployTargetType;
use arcs_yaml_parser::YamlShape;

use crate::logging::*;
use crate::server::utils::metadata::*;
use crate::server::responses::{ Response, Metadata };

use super::utils::api_types::incoming::Link;


async fn get_deployment_success_info(meta: &Metadata, ports: &Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Result<(YamlShape, String, Vec<Link>), String> {
    // Get YAML for sending challenge metadata
    let yaml_file = get_yaml_shape(meta).await?;
    debug!("Have a yaml: {yaml_file:#?}");


    // Challenge links
    let links = links::get_all_links(meta, &yaml_file, &ports)?;
    trace!("Built links for challenge metadata");
    debug!("{links:#?}");


    // Discord message body
    let disc_message = match discord_message::build_discord_message(meta, &ports, &links) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to build discord message for {}: {:#?}", meta.chall_name(), e);
            return Err("Failed to build discord message for challenge".to_string());
        },
    };
    trace!("Built discord message payload");
    debug!("{}", disc_message);


    // Turn deploy links into links the webhook can understand
    let links = links::into_webhook_links(links);
    debug!("Links: {links:#?}");

    Ok((yaml_file, disc_message, links))
}

pub async fn send_deployment_success(meta: &Metadata, ports: Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Result<(), String> {
    // reqwest client for contacting the webhook server
    let client = Client::new();
    
    // Get deployment success info
    let (yaml_file, disc_message, links) = get_deployment_success_info(meta, &ports).await?;

    // Create chall on webhook and send discord message
    deployment_success_req::deployment_success_message(&client, meta, &yaml_file, disc_message, links).await?;

    // Tell frontend to sync the new chall data
    sync::frontend_sync_message(&client, meta).await?;

    Ok(())
}

pub async fn send_deployment_failure(meta: &Metadata, err: String) -> Result<(), String> {
    // reqwest client for contacting the webhook server
    let client = Client::new();

    // Send discord message
    deployment_failure_req::deployment_failure_message(&client, meta, &err).await
}

pub async fn sync_metadata_with_webhook(meta: &Metadata, new_yaml: YamlShape) -> Response {
    // reqwest client for contacting the webhook server
    let client = Client::new();

    // reqwest client for contacting the webhook server
    match meta::metadata_update_message(&client, meta, &new_yaml).await {
        Ok(_) => Response::success(meta.clone(), None),
        Err(e) => Response::ise(&e, meta.clone()),
    }
}
