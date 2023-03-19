use serde::Serialize;
use serde_json::{ json, Value };
use crate::polling::{ PollingId, DeploymentStatus };
use super::Deploy;

#[derive(Debug, Copy, Clone, Serialize)]

/// **200** - Success
///  - 200 - Request received successfully
/// 
/// **51X** - Client Login Failures
/// - 510 - Docker Client Failure
/// - 511 - K8s Client Failure
/// 
/// **44X** - Polling ID Failures
/// - 440 - Polling ID already exists
/// - 441 - Polling ID has not been registered / invalid
/// 
/// **40X** - Endpoint Failures
/// - 404 - Endpoint does not exist
/// 
/// **55X** - Server Deploy Process Failures (SERVER AT FAULT)
/// - 550 + **subcode** - Server Deploy Process Failure
///
/// **45X** - Request Deploy Process Failures (CLIENT AT FAULT)
/// - 450 + **subcode** - Request Deploy Process Failure 
/// 
/// **50X** - Internal Server Error
/// - 500 - Unknown Internal Server Error Occurred
pub struct StatusCode { code: u64, message: &'static str }

impl StatusCode {
    pub const SUCCESS: Self = StatusCode { code: 200, message: "Request received successfully" };
    
    // endpoint failures
    pub const ENDPOINT_NO_EXIST_ERR: Self = StatusCode { code: 404, message: "Endpoint is not set up on the server" };

    // polling failures
    pub const POLLID_ALREADY_EXISTS_ERR: Self = StatusCode { code: 440, message: "Polling ID already exists" };
    pub const POLLID_INVAL_NOEXISTS_ERR: Self = StatusCode { code: 441, message: "Polling ID is unregistered" };
    
    // client login failures
    pub const DOCKER_LOGIN_ERR: Self = StatusCode { code: 510, message: "Failure initializing Docker client" };
    pub const K8SCLI_LOGIN_ERR: Self = StatusCode { code: 511, message: "Failure initializing Kubernetes client" };

    pub const UNKNOWN_ISE: Self = StatusCode { code: 500, message: "Unknown internal server error" };

    // deploy process failures
    pub fn server_deploy_process_err(subcode: u64, message: &'static str) -> Self {
        Self { code: 550 + subcode, message }
    }

    // deploy process failures
    pub fn req_deploy_process_err(subcode: u64, message: &'static str) -> Self {
        Self { code: 450 + subcode, message }
    }

    // deploy process failures
    pub fn custom(code: u64, message: &'static str) -> Self {
        Self { code, message }
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub meta: Metadata,
    pub internal_code: StatusCode,
}


// 200 success
impl Response {
    pub fn success(meta: Metadata, other_data: Option<serde_json::Value>) -> Self {
        Self {
            meta: Metadata {
                other_data,
                ..meta
            }, 
            internal_code: StatusCode::SUCCESS,
        }
    }
}

// 400 endpoint
impl Response {
    pub fn endpoint_err(bad_endpoint: &str, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data: Some(json!({
                    "bad_endpoint": bad_endpoint,
                })),
                ..meta
            }, 
            internal_code: StatusCode::ENDPOINT_NO_EXIST_ERR,
        }
    }
}

// 440 poll id
impl Response {
    pub fn poll_id_doesnt_exist(poll_id: PollingId, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data: Some(json!({
                    "unknown_poll_id": poll_id,
                })),
                ..meta
            }, 
            internal_code: StatusCode::POLLID_INVAL_NOEXISTS_ERR,
        }
    }
    pub fn poll_id_already_in_use(poll_id: PollingId, status: DeploymentStatus, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data: Some(json!({
                    "in_use_poll_id": poll_id,
                    "status_of_in_use": status,
                })),
                ..meta
            }, 
            internal_code: StatusCode::POLLID_ALREADY_EXISTS_ERR,
        }
    }
}

// 500 client login 
impl Response {
    pub fn docker_login_err(err: &str, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data: Some(json!({
                    "docker_error": err,
                })),
                ..meta
            }, 
            internal_code: StatusCode::DOCKER_LOGIN_ERR,
        }
    }

    pub fn k8s_login_err(err: &str, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data: Some(json!({
                    "k8s_error": err,
                })),
                ..meta
            }, 
            internal_code: StatusCode::K8SCLI_LOGIN_ERR,
        }
    }
}

// 400 endpoint
impl Response {
    pub fn server_deploy_process_err(subcode: u64, message: &'static str, other_data: Option<Value>, meta: Metadata) -> Self {
        Self {
            meta: Metadata {
                other_data,
                ..meta
            }, 
            internal_code: StatusCode::server_deploy_process_err(subcode, message),
        }
    }
}

impl Response {
    pub fn wrap(self) -> actix_web::web::Json<Self> {
        actix_web::web::Json(self)
    }
    #[deprecated(note = "Make a custom response function, don't use this")]
    pub fn custom(meta: Metadata, status_code: StatusCode) -> Self {
        Self { meta, internal_code: status_code }
    }

    pub fn ise(description: &str, meta: Metadata) -> Self {
        Self {
            internal_code: StatusCode::UNKNOWN_ISE,
            meta: Metadata {
                other_data: Some(json!({ "description": description })),
                ..meta
            },
        }
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    polling_id: PollingId,
    chall_name: String,
    endpoint_name: String,
    other_data: Option<serde_json::Value>,
}

impl From<&Deploy> for Metadata {
    fn from(deploy_input: &Deploy) -> Self {
        Self {
            polling_id: deploy_input.deploy_identifier,
            chall_name: deploy_input.chall_name.clone(),
            endpoint_name: deploy_input._type.to_uppercase(),
            other_data: None,
        }
    }
}
impl Metadata {
    pub fn poll_id(&self) -> PollingId {
        self.polling_id
    }
    pub fn chall_name(&self) -> &String {
        &self.chall_name
    }
    pub fn endpoint_name(&self) -> &String {
        &self.endpoint_name
    }
}

