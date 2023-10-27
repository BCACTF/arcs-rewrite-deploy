use serde_json::json;

use arcs_yaml_parser::YamlShape;

use crate::env::{webhook_address, deploy_token};
use crate::logging::*;
use crate::server::responses::Metadata;

async fn send_deployment_success(
    client: &reqwest::Client,
    meta: &Metadata,
    yaml: &YamlShape,
    disc_message: String,
    links: Vec<crate::server::utils::api_types::incoming::Link>,
) -> Result<reqwest::Response, String> {
    use crate::server::utils::api_types::incoming::*;

    let developer_discord_payload = ToDiscord::Developer(
        DeveloperDiscordMessage {
            data: json!({}),
            include_chall_writers: false,
            level: AlertLevel::Info,
            message: disc_message,
        }
    );
    trace!("Build discord payload");

    let sql_payload = ToSql::Chall(
        ChallQuery::Create {
            id: Some(meta.poll_id()),
            name: yaml.chall_name().to_string(),
            description: yaml.description().to_string(),
            points: yaml.points() as i32,

            authors: yaml.authors().to_vec(),
            hints: yaml.hints().to_vec(),
            categories: yaml.category_str_iter().map(str::to_string).collect::<Vec<String>>(),
            tags: vec![],
            links,

            source_folder: meta.chall_name().clone(),
            visible: yaml.visible(),

            flag: yaml.flag_str().to_string(),
        }
    );
    trace!("Build sql payload");

    let success_payload = Incoming {
        deploy: None,
        discord: Some(developer_discord_payload),
        frontend: None,
        sql: Some(sql_payload),
    };
    trace!("Build webhook DeploymentSuccess payload");


    // let chall_id: Uuid = meta.poll_id();
    // let frontend_payload = ToFrontend::Sync(IncomingSyncType::Chall(chall_id));
    // let sync_payload = Incoming {
    //     deploy: None,
    //     discord: None,
    //     frontend: Some(frontend_payload),
    //     sql: None,
    // };

    let response = client.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&success_payload)
        .send()
        .await;
    trace!("Sent DeploymentSuccess req");

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            error!("Error sending DeploymentSuccess message to webhook server");
            error!("Trace: {:#?}", err);
            return Err("Error sending DeploymentSuccess message to webhook server".to_string());
        }
    };
    trace!("Response recieved successfully");

    Ok(response)
}

async fn handle_deployment_success(
    response: reqwest::Response,
    meta: &Metadata,
) -> Result<(), String> {
    use crate::server::utils::api_types::outgoing::*;

    let status_code = response.status();

    match response.json::<Outgoing>().await {
        Ok(response) => {
            type FromSqlResult = ResultOfFromSqlOrFromSqlErr;
            type FromDiscordResult = ResultOfFromDiscordOrFromDiscordErr;

            let Some(FromSqlResult::Ok(FromSql::Chall(chall))) = response.sqll else {
                error!("Expected a challenge result from the SQL server, but got none, a bad result, or non-challenge result");
                return Err("SQL server returned an unexpected response".to_string());
            };
            if chall.id != meta.poll_id() {
                error!("SQL result did not have the same ID as the poll ID");
                return Err("SQL server returned a challenge with a different ID than the poll ID".to_string());
            };

            let Some(FromDiscordResult::Ok(_)) = response.disc else {
                error!("Expected a successful result from the SQL server, but got none or a bad result");
                return Err("Discord returned an undexpected result".to_string());
            };

            if !status_code.is_success() {
                error!("Despite good results otherwise, status code had an issue");
                return Err("Status code issue".to_string());
            }
        },
        Err(outgoing_error) => {
            error!("Error parsing response from webhook server");
            error!("Trace: {:#?}", outgoing_error);
            return Err("Error parsing response from webhook server".to_string());
        },
    };
    
    trace!("Successfully sent DeploymentSuccess message to webhook server");

    Ok(())
}

pub async fn deployment_success_message(
    client: &reqwest::Client,
    meta: &Metadata,
    yaml: &YamlShape,
    disc_message: String,
    links: Vec<crate::server::utils::api_types::incoming::Link>,
) -> Result<(), String> {
    trace!("Sending DeploymentSuccess message to SQL and Discord server");

    let response = send_deployment_success(client, meta, yaml, disc_message, links).await?;
    handle_deployment_success(response, meta).await
}
