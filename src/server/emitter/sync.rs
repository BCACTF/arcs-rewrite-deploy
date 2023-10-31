use crate::env::{webhook_address, deploy_token};
use crate::logging::*;
use crate::server::responses::Metadata;

async fn send_frontend_sync(
    client: &reqwest::Client,
    meta: &Metadata,
) -> Result<reqwest::Response, String> {
    use crate::server::utils::api_types::incoming::*;

    let chall_id = meta.poll_id();

    let frontend_payload = ToFrontend::Sync(SyncType::Chall(chall_id));
    let sync_payload = Incoming {
        deploy: None,
        discord: None,
        frontend: Some(frontend_payload),
        sql: None,
    };
    trace!("Build sync payload");

    let response = client.post(webhook_address())
        .bearer_auth(deploy_token())
        .json(&sync_payload)
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

async fn handle_frontend_sync(
    response: reqwest::Response,
    meta: &Metadata,
) -> Result<(), String> {
    use crate::server::utils::api_types::outgoing::*;

    let status_code = response.status();

    match response.json::<Outgoing>().await {
        Ok(response) => {
            use {
                FrontendResult::Success,
                FromFrontend::Synced,
                SyncType::Chall,
            };

            let Some(Success(Synced(Chall(chall_id)))) = response.frontend else {
                error!("Expected a chall id sync result from the Frontend server, but got none, a bad result, or non-chall id sync result");
                return Err("Frontend returned an unexpected response".to_string());
            };
            if chall_id != meta.poll_id() {
                error!("Frontend sync acknowledgement did not have the same ID as the poll ID");
                return Err("Frontend sync ack returned a challenge with a different ID than the poll ID".to_string());
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

pub async fn frontend_sync_message(
    client: &reqwest::Client,
    meta: &Metadata,
) -> Result<(), String> {
    trace!("Sending SyncDeploy message to Frontend server");

    let response = send_frontend_sync(client, meta).await?;
    handle_frontend_sync(response, meta).await
}
