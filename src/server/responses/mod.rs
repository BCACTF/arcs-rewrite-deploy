mod success;
mod client_err;
mod server_err;

use actix_web::{Responder, CustomizeResponder, web::Json};
use serde::Serialize;
use crate::polling::{ PollingId, DeploymentStatus, poll_deployment };
use super::Deploy;
use std::{borrow::Cow, time::Duration};


macro_rules! const_status_code {
    ($name:ident: $number:literal ($description:literal)) => {
        pub const $name: Self = StatusCode { code: $number, message: std::borrow::Cow::Borrowed($description) };
    };
}

/// ## Fields
/// - `Code` : `u64`
///     - Numeric code that is returned to the client
/// - `Message` : `&'static str`
///     - Status message that is returned to the client
/// 
/// ### 200 - Success
///  - `200` - Request received successfully
/// 
/// ### 40X - Endpoint Failures
/// - `404` - Endpoint does not exist
/// 
/// ### 44X - Polling ID Failures
/// - `440` - Polling ID already exists
/// - `441` - Polling ID has not been registered / invalid
///
/// ### 45X - Request Deploy Process Failures
/// - `450` + **subcode** - Request Deploy Process Failure 
/// 
/// ### 46X - File Upload Failures
/// - `460` - File Upload Failure
/// 
/// ### 50X - Internal Server Error
/// - `500` - Unknown Internal Server Error Occurred
/// - `501` - YAML Verification Error
/// 
/// ### 51X - Client Login Failures
/// - `510` - Docker Client Failure
/// - `511` - k8s Client Failure
/// 
/// ### 55X - Server Deploy Process Failures 
/// - `550` + **subcode** - Server Deploy Process Failure
/// 
/// ### 580 - Server Delete Failures
/// - `580` - Kubernetes Service/Deployment Deletion Failure
/// - `581` - Docker Image Deletion Failure 
#[derive(Debug, Clone, Serialize)]
pub struct StatusCode { code: u64, message: Cow<'static, str> }

impl StatusCode {
    const_status_code!(SUCCESS:  200 ("Request received successfully"));
    const_status_code!(ACCEPTED: 202 ("Started processing"));
    
    // Not found failures
    const_status_code!(ENDPOINT_NO_EXIST_ERR:      404 ("Endpoint is not set up on the server"));
    const_status_code!(CHALL_NAME_NO_EXISTS_ERR:   404 ("There is no challenge with this name"));
    const_status_code!(POLL_ID_INVAL_NOEXISTS_ERR: 404 ("Polling ID does not exist"));

    // Other Client Errors
    const_status_code!(POLL_ID_ALREADY_EXISTS_ERR: 409 ("Polling ID already exists"));
    const_status_code!(MODICATIONS_MISSING: 412 ("You must specify the modifications to make to the metadata"));


    // Metadata modification failures
    const_status_code!(MODICATIONS_FAILED:  500 ("Failed to modify the metadata. This MIGHT be an issue with the YAML file."));
    
    // client login failures
    const_status_code!(DOCKER_LOGIN_ERR: 500 ("Failure initializing Docker client"));
    const_status_code!(K8SCLI_LOGIN_ERR: 500 ("Failure initializing Kubernetes client"));

    // Internal Server Errors
    const_status_code!(UNKNOWN_ISE: 500 ("Unknown internal server error"));
    const_status_code!(IO_ERR:      500 ("Input output (filesystem) error"));
    const_status_code!(GIT_ERR:     500 ("Issues with the git management process"));

    // Deletion errors
    const_status_code!(K8S_SERVICE_DEPLOY_DEL_ERR: 500 ("Failure deleting Kubernetes resources"));
    const_status_code!(DOCKER_IMG_DEL_ERR:         500 ("Failure deleting Docker image"));
}



use super::utils::api_types::outgoing::FromDeploy as OutgoingFromDeploy;

pub struct Response(StatusCode, OutgoingFromDeploy);

impl Response {
    pub fn wrap(self) -> CustomizeResponder<Json<OutgoingFromDeploy>> {
        use actix_web::http::StatusCode as ActixStatusCode;

        let Self(status_code, body) = self;
        let code = status_code.code;
        let actix_code = ActixStatusCode::from_u16(code as u16).unwrap_or(ActixStatusCode::INTERNAL_SERVER_ERROR);

        let result_header = ("STATUS-TEXT", status_code.message.as_ref());

        actix_web::web::Json(body)
            .customize()
            .with_status(actix_code)
            .append_header(result_header)
    }
}


/// Struct that represents the data to be sent back in a response
/// 
/// ## Fields
/// - `poll_id` - PollingId to uniquely identify request
/// - `chall_name` - Challenge name that request pertained to
/// - `endpoint_name` - Endpoint that the request was sent/forwarded to
/// - `other_data` - `Option<serde_json::Value>` parameter that can be sent back to the client for additional information
#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    poll_id: PollingId,
    chall_name: String,
    status: DeploymentStatus,
    endpoint_name: String,
    other_data: Option<serde_json::Value>,
}

impl From<&Deploy> for Metadata {
    fn from(deploy_input: &Deploy) -> Self {
        let poll_id = deploy_input.deploy_identifier;
        let chall_name = deploy_input.chall_name.clone();
        let endpoint_name = deploy_input.__type.to_uppercase();

        let deployment = poll_deployment(poll_id).ok();
        let status = deployment.map(|d| d.status).unwrap_or_default();

        Self { poll_id, chall_name, endpoint_name, status, other_data: None }
    }
}
impl Metadata {
    pub fn poll_id(&self) -> PollingId {
        self.poll_id
    }
    pub fn chall_name(&self) -> &String {
        &self.chall_name
    }
    pub fn endpoint_name(&self) -> &String {
        &self.endpoint_name
    }
    pub fn status_is_unknown(&self) -> bool {
        matches!(self.status, DeploymentStatus::Unknown)
    }
    pub fn status(&self) -> &DeploymentStatus {
        &self.status
    }
}


use crate::polling::DeployStep;
use super::utils::api_types::outgoing::{ Status as WebhookStatus, Duration as WebhookDuration };

impl From<Duration> for WebhookDuration {
    fn from(value: Duration) -> Self {
        Self {
            secs: value.as_secs(),
            nanos: value.subsec_nanos(),
        }
    }
}

impl From<DeploymentStatus> for (WebhookStatus, WebhookDuration) {
    fn from(status: DeploymentStatus) -> Self {
        match status {
            DeploymentStatus::Unknown => (
                WebhookStatus::Unknown,
                Duration::ZERO.into(),
            ),
            DeploymentStatus::Success(time, _) => (
                WebhookStatus::Success,
                time.elapsed().into(),
            ),
            DeploymentStatus::InProgress(start_time, step) => (
                match step {
                    DeployStep::Building => WebhookStatus::Building,
                    DeployStep::Pushing => WebhookStatus::Pushing,
                    DeployStep::Pulling => WebhookStatus::Pulling,
                    DeployStep::Deploying => WebhookStatus::Uploading,
                },
                start_time.elapsed().into(),
            ),
            DeploymentStatus::Failure(time, _) => (
                WebhookStatus::Failure,
                time.elapsed().into(),
            ),
        }
    }
}
