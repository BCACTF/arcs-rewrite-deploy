use actix_web::{Responder, CustomizeResponder, web::Json};
use arcs_yaml_parser::YamlVerifyError;
use serde::Serialize;
use serde_json::{ json, Value };
use uuid::Uuid;
use crate::polling::{ PollingId, DeploymentStatus, poll_deployment };
use super::Deploy;
use std::{borrow::Cow, time::Duration};




const fn cowify(s: &'static str) -> Cow<'static, str> { Cow::Borrowed(s) }

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
    pub const SUCCESS: Self = StatusCode { code: 200, message: cowify("Request received successfully") };
    
    // endpoint failures
    pub const ENDPOINT_NO_EXIST_ERR: Self = StatusCode { code: 404, message: cowify("Endpoint is not set up on the server") };

    // No challenge of that name
    pub const CHALL_NAME_NO_EXISTS_ERR: Self = StatusCode { code: 430, message: cowify("There is no challenge with this name") };

    // polling failures
    pub const POLLID_ALREADY_EXISTS_ERR: Self = StatusCode { code: 440, message: cowify("Polling ID already exists") };
    pub const POLLID_INVAL_NOEXISTS_ERR: Self = StatusCode { code: 441, message: cowify("Polling ID is unregistered") };

    // metadata modification failures
    pub const MODICATIONS_MISSING: Self = StatusCode { code: 460, message: cowify("You must specify the modifications to make to the metadata") };
    pub const MODICATIONS_FAILED: Self = StatusCode { code: 561, message: cowify("Failed to modify the metadata. This MIGHT be an issue with the file.") };
    
    // client login failures
    pub const DOCKER_LOGIN_ERR: Self = StatusCode { code: 510, message: cowify("Failure initializing Docker client") };
    pub const K8SCLI_LOGIN_ERR: Self = StatusCode { code: 511, message: cowify("Failure initializing Kubernetes client") };

    // Internal Server Errors
    pub const UNKNOWN_ISE: Self = StatusCode { code: 500, message: cowify("Unknown internal server error") };

    // deletion errors
    pub const K8S_SERVICE_DEPLOY_DEL_ERR: Self = StatusCode { code: 580, message: cowify("Failure deleting Kubernetes resources") };
    pub const DOCKER_IMG_DEL_ERR: Self = StatusCode { code: 580, message: cowify("Failure deleting Docker image") };

    // deploy process failures
    pub fn server_deploy_process_err(subcode: u64, message: &'static str) -> Self {
        Self { code: 550 + subcode, message: message.into() }
    }

    // deploy process failures
    pub fn req_deploy_process_err(subcode: u64, message: &'static str) -> Self {
        Self { code: 450 + subcode, message: message.into() }
    }
    // TODO --> document and organize this
    pub fn yaml_verify_err(yaml_err: arcs_yaml_parser::YamlVerifyError, message: &'static str) -> Self {
        Self { code: 501, message: format!("{}. YamlError: {}", message, yaml_err).into() }
    }

    // file errors
    pub fn file_upload_err(subcode: u64, message: &'static str) -> Self {
        Self { code: 460 + subcode, message: message.into() }
    }

    // custom failure
    pub fn custom(code: u64, message: &'static str) -> Self {
        Self { code, message: message.into() }
    }
}


/// Struct that represents the data to be sent back in a response
/// 
/// ## Fields
/// - `meta` - [Metadata]
///     - Metadata that is sent back to the client
/// - `internal_code` - [StatusCode]
///     - Internal code that is sent back to the client, uses status codes defined in [StatusCode]
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub internal_code: StatusCode,

    pub status: &'static str,
    pub status_time: Duration,
    pub chall_name: Option<String>,
    pub poll_id: Uuid,
    pub data: Option<serde_json::Value>,
}

// 200 success
impl Response {
    pub fn success(meta: Metadata, other_data: Option<serde_json::Value>) -> Self {
        Self {
            internal_code: StatusCode::SUCCESS,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,
            data: other_data,
        }
    }
}

// 400 endpoint
impl Response {
    pub fn endpoint_err(bad_endpoint: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::ENDPOINT_NO_EXIST_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: Some(json!({
                "bad_endpoint": bad_endpoint,
            })),
        }
    }
}

// 440 poll id
impl Response {
    pub fn chall_doesnt_exist(chall_name: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::CHALL_NAME_NO_EXISTS_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(chall_name.to_string()),
            poll_id: meta.poll_id,

            data: Some(json!({
                "unknown_challenge": chall_name,
            })),
        }
    }

    pub fn poll_id_doesnt_exist(poll_id: PollingId, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::POLLID_INVAL_NOEXISTS_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,


            data: Some(json!({
                "unknown_poll_id": poll_id,
            })),
        }
    }
    pub fn poll_id_already_in_use(poll_id: PollingId, status: DeploymentStatus, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::POLLID_ALREADY_EXISTS_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,


            data: Some(json!({
                "in_use_poll_id": poll_id,
                "status_of_in_use": status,
            }))
        }
    }

    pub fn modifications_missing(meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::MODICATIONS_MISSING,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: None,
        }
    }

    pub fn modifications_failed(meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::MODICATIONS_FAILED,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: None,
        }
    }
}

// 500 client login 
impl Response {
    pub fn docker_login_err(err: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::DOCKER_LOGIN_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,


            data: Some(json!({
                "err": err,
            })),
        }
    }

    pub fn k8s_login_err(err: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::K8SCLI_LOGIN_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,


            data: Some(json!({
                "err": err,
            })),
        }
    }
}

// 400 endpoint
impl Response {
    pub fn server_deploy_process_err(subcode: u64, message: &'static str, other_data: Option<Value>, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::server_deploy_process_err(subcode, message),

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: other_data,
        }
    }

    pub fn file_upload_process_err(subcode: u64, message: &'static str, other_data: Option<Value>, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::file_upload_err(subcode, message),

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: other_data,
        }
    }
}

// 580 endpoints
impl Response {
    pub fn k8s_service_deploy_del_err(other_data: impl serde::Serialize, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::K8S_SERVICE_DEPLOY_DEL_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: Some(json!({ "err": other_data })),
        }
    }
    pub fn docker_img_del_err(other_data: impl serde::Serialize, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::DOCKER_IMG_DEL_ERR,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: Some(json!({ "err": other_data })),
        }
    }
}

impl Response {
    /// Convenience function that wraps the response in a `actix_web::web::Json` object to return to the client
    pub fn wrap(self) -> CustomizeResponder<Json<Self>> {
        use actix_web::http::StatusCode as ActixStatusCode;
        let code = self.internal_code.code;
        let actix_code = ActixStatusCode::from_u16(code as u16).unwrap_or(ActixStatusCode::INTERNAL_SERVER_ERROR);
        actix_web::web::Json(self).customize().with_status(actix_code)
    }
    #[deprecated(note = "Make a custom response function, don't use this")]
    pub fn custom(meta: Metadata, status_code: StatusCode) -> Self {
        Self {
            internal_code: status_code,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: meta.other_data,
        }
    }

    pub fn ise(description: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::UNKNOWN_ISE,

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: Some(json!({ "description": description })),
        }
    }
}

impl From<(YamlVerifyError, Metadata)> for Response {
    fn from((yaml_error, meta): (YamlVerifyError, Metadata)) -> Self {
        Self {
            internal_code: StatusCode::yaml_verify_err(yaml_error, "Failed to verify yaml"),

            status_time: meta.status.since_last_change(),
            status: meta.status.get_str(),
            chall_name: Some(meta.chall_name),
            poll_id: meta.poll_id,

            data: meta.other_data,
        }
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
}

