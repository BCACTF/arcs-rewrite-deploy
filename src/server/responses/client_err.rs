use uuid::Uuid;

use crate::server::utils::api_types::outgoing::{ DeploymentStatus, FromDeploy, Status };

use super::{Metadata, Response, StatusCode};


impl Response {
    pub fn endpoint_doesnt_exist_err(meta: Metadata, endpoint_name: &str) -> Self {
        let chall_name = Some(endpoint_name.to_string());
        let poll_id = meta.poll_id();
        
        Self(
            StatusCode::ENDPOINT_NO_EXIST_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("Endpoint {endpoint_name:?} doesn't exist")),
            }),
        )
    }

    pub fn err_chall_name_doesnt_exist(meta: Metadata, name: &str) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        
        Self(
            StatusCode::CHALL_NAME_NO_EXISTS_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("Challenge {name:?} doesn't exist")),
            }),
        )
    }
    pub fn err_poll_id_doesnt_exist(meta: Metadata, poll_id: Uuid) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        
        Self(
            StatusCode::POLL_ID_INVAL_NOEXISTS_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("Poll ID {poll_id} doesn't exist.")),
            }),
        )
    }

    pub fn poll_id_in_use(meta: Metadata, poll_id: Uuid, status: crate::polling::DeploymentStatus) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        
        let (status, status_time) = status.into();

        Self(
            StatusCode::POLL_ID_ALREADY_EXISTS_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status,
                status_time,
                err_msg: Some(format!("Poll ID {poll_id} already exists. Status has been sent.")),
            }),
        )
    }

    pub fn modifications_missing(meta: Metadata) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        
        Self(
            StatusCode::MODICATIONS_MISSING,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some("No modifications were provided.".to_string()),
            }),
        )
    }
}
