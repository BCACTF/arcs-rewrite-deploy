mod deployment_success_req;
mod sync;

use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

use arcs_yaml_parser::deploy::structs::{DeployLink, DeployTargetType};
use arcs_yaml_parser::YamlShape;


use crate::logging::*;
use crate::env::{ webhook_address, deploy_token };
use crate::server::responses::{ Response, Metadata };

use crate::server::utils::metadata::{
    get_yaml_shape,
    container_links::links_from_port_listing,
    discord_message::build_discord_message,
    links::get_static_file_links,
};

use crate::server::utils::api_types::incoming::{ Link, LinkType };

pub fn get_db_id(json: serde_json::Value) -> Option<Uuid> {
    let id_str = json.get("sql")?.get("id")?.as_str()?;
    Uuid::parse_str(id_str).ok()
}

// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn send_deployment_success(meta: &Metadata, ports: Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Result<(), String> {
    let client = Client::new();
    
    let yaml_file = get_yaml_shape(meta).await?;
    debug!("Have a yaml: {yaml_file:#?}");

    
    let static_files = match get_static_file_links(meta, &yaml_file) {
        Ok(files) => files,
        Err(e) => {
            error!("Failed to get static file links for {}: {:#?}", meta.chall_name(), e);
            return Err("Failed to get static file links for challenge".to_string());
        }
    };

    let mut complete_links : Vec<DeployLink> = static_files
        .iter()
        .map(|static_link| {
            DeployLink {
                deploy_target: DeployTargetType::Static,
                link: static_link.to_string(),
            }}).collect();
    
    complete_links.extend(links_from_port_listing(&ports).into_iter());

    trace!("Built links for challenge metadata");
    debug!("{:#?}", complete_links);

    let disc_message = match build_discord_message(meta, &ports, &complete_links) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to build discord message for {}: {:#?}", meta.chall_name(), e);
            return Err("Failed to build discord message for challenge".to_string());
        },
    };
    trace!("Built discord message payload");
    debug!("{}", disc_message);


    let links = complete_links
        .into_iter()
        .map(|link| {
            Link {
                location: link.link.clone(),
                type_: match link.deploy_target {
                    DeployTargetType::Static => LinkType::Static,
                    DeployTargetType::Nc => LinkType::Nc,
                    DeployTargetType::Admin => LinkType::Admin,
                    DeployTargetType::Web => LinkType::Web,
                },
            }
        })
        .collect::<Vec<Link>>();
    debug!("Links: {links:#?}");

    deployment_success_req::deployment_success_message(&client, meta, &yaml_file, disc_message, links).await?;
    sync::frontend_sync_message(&client, meta).await?;

    Ok(())
}

// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn send_deployment_failure(meta: &Metadata, err: String) -> Result<(), String> {
    let poll_id = meta.poll_id();
    let emitter = Client::new();

    let jsonbody = json!({
        "discord": {
            "__type": "developer",
            "level": "WARN",
            "message": format!("Failed to deploy **{}**\n({})\nCheck logs for more info", meta.chall_name(), poll_id),
            "data": {},
            "include_chall_writers": false,
        },
    });

    info!("Sending deployment failure (err: {err:?})");

    let response = emitter.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&jsonbody)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
            } else {
                error!("Error sending DeploymentFailure message to webhook server : Bad status code returned");
                error!("Trace: {:#?}", resp);

                if resp.status() == 401 {
                    warn!("Webhook server returned 401 Unauthorized. Check that the DEPLOY_SERVER_AUTH_TOKEN is correct");
                }

                Err("Error sending DeploymentFailure message to webhook server".to_string())
            }
        },
        Err(err) => {
            error!("Error sending DeploymentFailure message to webhook server");
            error!("Trace: {:#?}", err);
            Err("Error sending DeploymentFailure message to webhook server".to_string())
        }
    }
}


// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn sync_metadata_with_webhook(meta: &Metadata, new_yaml: YamlShape) -> Response {
    let emitter = Client::new();

    let sql_payload = json!({
        "__type": "chall",
        "query_name": "update",

        "id": meta.poll_id(),
        "name": &new_yaml.chall_name(),
        "description": &new_yaml.description(),
        "points": &new_yaml.points(),
        "categories": &new_yaml.category_str_iter().collect::<Vec<&str>>(),
        "tags": [], // TODO --> add tags in YamlShape
        "visible": new_yaml.visible(),
    });
    let jsonbody = json!({
        "sql": sql_payload,
    });

    trace!("built JSON body");

    let response = emitter.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&jsonbody)
        .send()
        .await;

    trace!("Sent req");

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Successfully sent DeploymentSuccess message to webhook server");
                
                Response::success(meta.clone(), None)
            } else {
                error!("Error sending DeploymentSuccess message to webhook server : Bad status code returned");
                error!("Trace: {:#?}", resp);

                if resp.status() == 401 {
                    warn!("Webhook server returned 401 Unauthorized. Check that the DEPLOY_SERVER_AUTH_TOKEN is correct");
                }

                Response::ise("Error sending DeploymentSuccess message to webhook server", meta.clone())
            }
        },
        Err(err) => {
            error!("Error sending DeploymentSuccess message to webhook server");
            error!("Trace: {:#?}", err);

            Response::ise("Error sending DeploymentSuccess message to webhook server", meta.clone())

        }
    }
}
