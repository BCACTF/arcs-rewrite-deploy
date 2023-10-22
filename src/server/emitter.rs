use reqwest::Client;
use serde_json::json;
use uuid::Uuid;
use std::fmt::Write;

use arcs_yaml_parser::deploy::structs::{DeployLink, DeployTargetType};
use arcs_yaml_parser::files::structs::ContainerType;
use arcs_yaml_parser::{File, YamlShape};

use arcs_deploy_static::fetch_chall_yaml;

use crate::logging::*;
use crate::env::{webhook_address, deploy_token, s3_display_address, deploy_address};
use crate::server::responses::Metadata;

use super::responses::Response;


pub async fn get_static_file_links(meta: &Metadata, yaml: &YamlShape) -> Result<Vec<String>, String> {
    let mut static_file_links : Vec<String> = Vec::new();
    let files = yaml
        .file_iter()
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<File>>();
    let chall = meta.chall_name().trim_matches('/');
    let base = s3_display_address().trim_matches('/');

    // TODO --> improve error messages on these branches 
    for file in files {
        info!("FILE: {:?}", file);
        if let Some(containertype) = file.container() {
            match containertype {
                ContainerType::Nc => {
                    if let Some(file_path) = file.path().to_str() {
                        if let Some((_, filename)) = file_path.rsplit_once('/') {
                            static_file_links.push(format!("{base}/{chall}/{filename}"));
                        } else {
                            return Err("Failed to parse file path for file".to_string());
                        }
                    } else {
                        return Err("Failed to find file path for file".to_string());
                    }
                }
                // If in the future there are other weird container files, add more branches here
                _ => {
                    if let Some(file_path) = file.path().to_str() {
                        info!("FILE PATH: {:?}", file_path.rsplit_once('/'));
                        if let Some((_, name)) = file_path.rsplit_once('/') {
                            static_file_links.push(format!("{base}/{chall}/{name}"));
                        } else {
                            return Err("Failed to parse file path for file".to_string());
                        }
                    } else {
                        return Err("Failed to parse file path".to_string());
                    }
                }
            }
        } else {
            info!("ADDING REGULAR STATIC FILE");
            if let Some(file_path) = file.path().to_str() {
                let file_name = file_path.rsplit_once('/').map(|(_, name)| name).unwrap_or(file_path);
                info!("FILE NAME: {:?}", file_name);
                static_file_links.push(format!("{base}/{chall}/{file_name}"));
            } else {
                return Err("Failed to parse file path".to_string());
            }
        }
    }
    info!("STATIC FILE LINKS: {:?}", static_file_links);
    Ok(static_file_links)
}


pub fn get_db_id(json: serde_json::Value) -> Option<Uuid> {
    let id_str = json.get("sql")?.get("id")?.as_str()?;
    Uuid::parse_str(id_str).ok()
}

// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn send_deployment_success(meta: &Metadata, ports: Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Result<(), String> {
    let poll_id = meta.poll_id();
    let emitter = Client::new();
    
    let yaml_file = if let Some(yaml_file) = fetch_chall_yaml(meta.chall_name()) {
        match yaml_file {
            Ok(yaml) => yaml,
            Err(e) => {
                error!("Failed to parse chall.yaml for {}: {:#?}", meta.chall_name(), e);
                return Err("Failed to parse chall.yaml for challenge".to_string());
            }
        }
    } else {
        error!("Failed to find chall.yaml for challenge: {}", meta.chall_name());
        return Err("Failed to find chall.yaml for challenge".to_string());
    };

    println!("Have a yaml: {yaml_file:#?}");

    let static_files = match get_static_file_links(meta, &yaml_file).await {
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

    if let Some(ports) = ports.as_ref() {
        for targettype_ports in ports {
            if targettype_ports.0 == DeployTargetType::Nc {
                if let Some((_, ip)) = deploy_address().split_once("://") {
                    for port in targettype_ports.1.iter() {
                        
                        let ip = "challs.bcactf.com"; // TODO --> remove this after event

                        complete_links.push(
                            DeployLink {
                                deploy_target: targettype_ports.0,
                                link: format!("{} {}", ip, port),
                            }
                        );
                    }
                };
            } else {
                for port in targettype_ports.1.iter() {
                    // let sanitized_address = deploy_address().trim_matches('/'); // this is a hack, don't be like me
                    let sanitized_address = "challs.bcactf.com";

                    complete_links.push(
                        DeployLink {
                            deploy_target: targettype_ports.0,
                            link: format!("{}:{}", sanitized_address, port),
                        }
                    );
                }
            }
        }
    }

    let mut disc_message = String::with_capacity(240);

    if let Some(ports) = ports.as_ref() {
        write!(disc_message, "Successfully deployed **{}** on port(s) {ports:?}", meta.chall_name())
    } else {
        write!(disc_message, "Successfully deployed **{}**. No ports provided", meta.chall_name())
    }.map_err(|e| e.to_string())?;

    if !complete_links.is_empty() {
        // TODO --> Maybe make this a bit nicer, isn't really the best way of doing this *probably*
        // Also, for netcat servers, the server this sends out is in the form of an http link which is... not correct.
        for link_to_file in &complete_links {
            let link_to_file_link = &link_to_file.link;

            let server_type = link_to_file.deploy_target.resource_type();
            write!(disc_message, "{server_type} at: {link_to_file_link}").map_err(|e| e.to_string())?;
        }
    }

    info!("{:#?}", complete_links);
    let developer_discord_payload = json!({
        "__type": "developer",
        "level": "INFO",
        "message": disc_message,
        "data": {},
        "include_chall_writers": false,
    });
    let sql_payload = json!({
        "__type": "chall",
        "query_name": "create",

        "id": meta.poll_id(),
        "name": &yaml_file.chall_name(),
        "description": &yaml_file.description(),
        "points": &yaml_file.points(),

        "authors": &yaml_file.authors(),
        "hints": &yaml_file.hints(),
        "categories": &yaml_file.category_str_iter().collect::<Vec<&str>>(),
        "tags": [], // TODO --> add tags in YamlShape
        "links": complete_links,

        "source_folder": meta.chall_name(), // TODO --> Add correct source_folder name, right now assumes chall_name
        "visible": &yaml_file.visible(),
        
        "flag": &yaml_file.flag_str(),
    });
    let jsonbody = json!({
        "sql": sql_payload,
        "discord": developer_discord_payload,
    });

    let response = emitter.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&jsonbody)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Successfully sent DeploymentSuccess message to webhook server");
                info!("Sending DeploymentSuccess message to Frontend server");
                
                // let chall_id : Uuid = if let Some(chall_id) = resp.json().await.ok().and_then(get_db_id) {
                //     chall_id
                // } else { 
                //     // TODO --> add better handling of the error here
                //     error!("FAILED"); 
                //     return Err("Failed to get database id from webhook server".to_string());
                // };
                let chall_id : Uuid = meta.poll_id();
                

                let frontend_body = json!({
                    "frontend": {
                        "__type": "sync",
                        "__sync_type": "chall",
                        "id": chall_id,
                    },
                });

                info!("made json body");

                let frontend_response = emitter.post(webhook_address())
                    .bearer_auth(deploy_token())
                    .json(&frontend_body)
                    .send()
                    .await;

                info!("sent out request to frontend");

                match frontend_response {
                    Ok(response) => {
                        if response.status().is_success() {
                            info!("Successfully sent SyncSuccessDeploy message to Frontend server");
                            Ok(())
                        } else {
                            error!("Error sending SyncSuccessDeploy message to Frontend server : Bad status code returned");
                            error!("Trace: {:#?}", response);

                            if response.status() == 401 {
                                warn!("Frontend server returned 401 Unauthorized. Check that the DEPLOY_SERVER_AUTH_TOKEN is correct");
                            }

                            Err("Error sending SyncSuccessDeploy message to Frontend server".to_string())
                        }
                    },
                    Err(err) => {
                        error!("Error sending SyncSuccessDeploy message to Frontend server");
                        error!("Trace: {:#?}", err);
                        Err("Error sending SyncSuccessDeploy message to Frontend server".to_string())
                    }
                }
            } else {
                error!("Error sending DeploymentSuccess message to webhook server : Bad status code returned");
                error!("Trace: {:#?}", resp);

                if resp.status() == 401 {
                    warn!("Webhook server returned 401 Unauthorized. Check that the DEPLOY_SERVER_AUTH_TOKEN is correct");
                }

                Err("Error sending DeploymentSuccess message to webhook server".to_string())
            }
        },
        Err(err) => {
            error!("Error sending DeploymentSuccess message to webhook server");
            error!("Trace: {:#?}", err);
            Err("Error sending DeploymentSuccess message to webhook server".to_string())

        }
    }
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


