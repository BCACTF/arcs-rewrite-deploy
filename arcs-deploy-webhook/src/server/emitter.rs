use reqwest::Client;

use crate::logging::*;

use serde_json::json;

use lazy_static::lazy_static;

use super::responses::Metadata;

lazy_static!(

    static ref WEBHOOK_SERVER_URL: String = std::env::var("WEBHOOK_SERVER_URL").expect("WEBHOOK_SERVER_URL must be set");
    static ref DEPLOY_SERVER_AUTH_TOKEN: String = std::env::var("DEPLOY_SERVER_AUTH_TOKEN").expect("DEPLOY_SERVER_AUTH_TOKEN must be set");

);

// TODO - validate return types of this function
// TODO - Actually make the SQL, Main, Discord branches send out the correct information
pub async fn send_deployment_success(meta: &Metadata, ports: Vec<i32>) -> Result<(), String> {
    let poll_id = meta.poll_id();
    
    let emitter = Client::new();

    let jsonbody = json!(
        {
            "_type": "DeploymentSuccess",
            "targets": {
                "discord": {
                    "content": format!("Successfully deployed {} on port {:?}", meta.chall_name(), ports),
                    "Ports": ports,
                    "urgency": "low"
                },
                "frontend": {
                    "PollID": poll_id,
                    "Ports": ports
                },
                "sql": {
                    "query": format!("INSERT INTO deployments (poll_id, ports) VALUES ({})", poll_id),
                    "Ports": ports
                }
            }
        }
    );

    let response = emitter.post(WEBHOOK_SERVER_URL.as_str())
        .bearer_auth(DEPLOY_SERVER_AUTH_TOKEN.as_str())
        .json(&jsonbody)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                return Ok(());
            } else {
                error!("Error sending DeploymentSuccess message to webhook server : Bad status code returned");
                error!("Trace: {:#?}", resp);

                if resp.status() == 401 {
                    warn!("Webhook server returned 401 Unauthorized. Check that the DEPLOY_SERVER_AUTH_TOKEN is correct");
                }

                return Err(format!("Error sending DeploymentSuccess message to webhook server"));
            }
        },
        Err(err) => {
            error!("Error sending DeploymentSuccess message to webhook server");
            error!("Trace: {:#?}", err);
            return Err(format!("Error sending DeploymentSuccess message to webhook server"));

        }
    }
}