use serde_json::json;

use crate::env::{webhook_address, deploy_token};
use crate::logging::*;
use crate::server::responses::Metadata;

async fn send_deployment_failure(
    client: &reqwest::Client,
    meta: &Metadata,
    err: &str,
) -> Result<reqwest::Response, String> {
    use crate::server::utils::api_types::incoming::*;

    let discord_payload = ToDiscord::Developer(
        DeveloperDiscordMessage {
            data: json!({}),
            level: AlertLevel::Warn,
            message: format!("Failed to deploy **{}**\n({})\nCheck logs for more info", meta.chall_name(), meta.poll_id()),
            include_chall_writers: false,
        }
    );
    let fail_payload = Incoming {
        deploy: None,
        discord: Some(discord_payload),
        frontend: None,
        sql: None,
    };
    trace!("Build DeploymentFailure payload");

    debug!("Sending DeploymentFailure message: {err}");

    let response = client.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&fail_payload)
        .send()
        .await;
    trace!("Sent DeploymentSuccess req");

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            error!("Error sending DeploymentFailure message to webhook server");
            error!("Trace: {:#?}", err);
            return Err("Error sending DeploymentFailure message to webhook server".to_string());
        }
    };
    trace!("Response recieved successfully");

    Ok(response)
}

async fn handle_deployment_failure(
    response: reqwest::Response,
) -> Result<(), String> {
    use crate::server::utils::api_types::outgoing::*;

    let status_code = response.status();

    match response.json::<Outgoing>().await {
        Ok(response) => {

            let Some(DiscordResult::Success(_)) = response.discord else {
                error!("Expected a successful result from Discord, but got none or a bad result");
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

pub async fn deployment_failure_message(
    client: &reqwest::Client,
    meta: &Metadata,
    err: &str,
) -> Result<(), String> {
    trace!("Sending DeploymentSuccess message to SQL and Discord server");

    let response = send_deployment_failure(client, meta, err).await?;
    handle_deployment_failure(response).await
}
