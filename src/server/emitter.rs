use arcs_yaml_parser::deploy::structs::{DeployTargetType, DeployLink};
use arcs_yaml_parser::files::structs::ContainerType;
use arcs_yaml_parser::{File, YamlShape};
use reqwest::Client;
use serde_json::json;

use crate::logging::*;
use crate::env::{webhook_address, deploy_token, s3_display_address, deploy_address};
use crate::server::responses::Metadata;

use arcs_deploy_static::fetch_chall_yaml;

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
            if let Some(containertype) = file.container() {
                match containertype {
                    ContainerType::Nc => {
                        if let Some(file_path) = file.path().to_str() {
                            if let Some((_, filename)) = file_path.rsplit_once("/") {
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
                            info!("FILE PATH: {:?}", file_path.rsplit_once("/"));
                            if let Some((_, name)) = file_path.rsplit_once("/") {
                                static_file_links.push(format!("{base}/{chall}/{name}"));
                            } else {
                                return Err("Failed to parse file path for file".to_string());
                            }
                        } else {
                            return Err("Failed to parse file path".to_string());
                        }
                    }
                }
            }
        }

        Ok(static_file_links)
}

// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn send_deployment_success(meta: &Metadata, ports: Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Result<(), String> {
    let poll_id = meta.poll_id();
    let emitter = Client::new();
    let mut discord_message_content: String;
    
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
                    let sanitized_address = deploy_address().trim_matches('/');
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

    if let Some(ports) = ports.as_ref() {
        discord_message_content = format!("Successfully deployed **{}** on port {:?}", meta.chall_name(), ports);
    } else {
        discord_message_content = format!("Successfully deployed **{}**. No ports provided", meta.chall_name());
    }

    if !complete_links.is_empty() {
        // TODO --> Maybe make this a bit nicer, isn't really the best way of doing this *probably*
        // Also, for netcat servers, the server this sends out is in the form of an http link which is... not correct.
        for link_to_file in &complete_links {
            let link_to_file_link = &link_to_file.link;
            match link_to_file.deploy_target {
                DeployTargetType::Nc => {
                    discord_message_content.push_str(format!("\nNetcat server at: {}", link_to_file_link).as_str()); 
                }
                DeployTargetType::Web => {
                    discord_message_content.push_str(format!("\nWeb server at: {}", link_to_file_link).as_str()); 
                }
                DeployTargetType::Admin => {
                    discord_message_content.push_str(format!("\nAdmin bot server at: {}", link_to_file_link).as_str()); 
                }
                DeployTargetType::Static => {
                    discord_message_content.push_str(format!("\nStatic file at: {}", link_to_file_link).as_str()); 
                }
            }
        }
    }

    let jsonbody = json!(
        {
            "_type": "DeploymentSuccess",
            "targets": {
                "discord": {
                    "content": discord_message_content,
                    "urgency": "low"
                },
                "frontend": {
                    "PollID": poll_id,
                },
                "sql": {
                    "section": "challenge",
                    "query": {
                        "__tag": "create",
                        "name": meta.chall_name(),
                        "description": &yaml_file.description(),
                        "points": &yaml_file.points(),
                        "authors": &yaml_file.authors(),
                        "hints": &yaml_file.hints(),
                        "categories": &yaml_file.category_str_iter().into_iter().collect::<Vec<&str>>(),
                        "tags": [], // TODO --> add tags in YamlShape
                        "links": complete_links,
                        "source_folder": meta.chall_name(), // TODO --> Add correct source_folder name, right now assumes chall_name
                        "visible": &yaml_file.visible(),
                        "flag": &yaml_file.flag_str()
                    }
                }
            }
        }
    );

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

    let jsonbody = json!(
        {
            "_type": "DeploymentFailure",
            "targets": {
                "discord": {
                    "content": format!("Failed to deploy **{}**\n({})\nCheck logs for more info", meta.chall_name(), poll_id),
                    "urgency": "medium"
                },
                "frontend": {
                    "PollID": poll_id,
                    "message": format!("Failed to deploy {}. Check logs for info.", meta.chall_name()),
                    "trace": err
                }
            }
        }
    );

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