use arcs_yaml_parser::YamlShape;

use crate::env::{webhook_address, deploy_token};
use crate::logging::*;
use crate::server::responses::Metadata;

async fn send_metadata_update(
    client: &reqwest::Client,
    meta: &Metadata,
    new_yaml: &YamlShape,
) -> Result<reqwest::Response, String> {
    use crate::server::utils::api_types::incoming::*;

    let sql_payload = ToSql::Chall(ChallQuery::Update {
        id: meta.poll_id(),

        name: Some(new_yaml.chall_name().to_string()),
        description: Some(new_yaml.description().to_string()),
        points: Some(new_yaml.points() as i32),
        categories: Some(new_yaml.category_str_iter().map(|s| s.to_string()).collect()),
        tags: Some(vec![]), // TODO --> add tags in YamlShape
        visible: Some(new_yaml.visible()),
        
        authors: None,
        hints: None,
        links: None,
        source_folder: None,
    });

    let update_metadata_payload = Incoming {
        deploy: None,
        discord: None,
        frontend: None,
        sql: Some(sql_payload),
    };

    trace!("Built UpdateMetadata payload");

    let response = client.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&update_metadata_payload)
        .send()
        .await;
    trace!("Sent UpdateMetadata req");

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            error!("Error sending UpdateMetadata message to webhook server");
            error!("Trace: {:#?}", err);
            return Err("Error sending UpdateMetadata message to webhook server".to_string());
        }
    };
    trace!("Response recieved successfully");

    Ok(response)
}

async fn handle_metadata_update(
    response: reqwest::Response,
    meta: &Metadata,
) -> Result<(), String> {
    use crate::server::utils::api_types::outgoing::*;

    let status_code = response.status();

    match response.json::<Outgoing>().await {
        Ok(response) => {

            let Some(SqlResult::Success(FromSql::Chall(chall))) = response.sql else {
                error!("Expected an updated challenge from the SQL server, but got none, a bad result, or non-chall sync result");
                return Err("SQL server returned an unexpected response".to_string());
            };
            if chall.id != meta.poll_id() {
                error!("SQL Server return type did not have the same ID as the poll ID");
                return Err("SQL Server returned a challenge with a different ID than the poll ID".to_string());
            }

            if !status_code.is_success() {
                error!("Despite good results otherwise, status code had an issue");
                return Err("Status code issue".to_string());
            }
        },
        Err(outgoing_err) => {
            error!("Error parsing response from webhook server");
            debug!("Error: {:#?}", outgoing_err);
            return Err("Error parsing response from webhook server".to_string());
        }
    }

    Ok(())
}

pub async fn metadata_update_message(
    client: &reqwest::Client,
    meta: &Metadata,
    yaml: &YamlShape
) -> Result<(), String> {
    trace!("Sending UpdateMetadata message to Frontend server");

    let response = send_metadata_update(client, meta, yaml).await?;
    handle_metadata_update(response, meta).await
}
