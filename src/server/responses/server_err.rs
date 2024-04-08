use std::fmt::Display;

use crate::server::utils::api_types::outgoing::{ DeploymentStatus, FromDeploy, Status };

use super::{Metadata, Response, StatusCode};


impl Response {
    pub fn err_modifications_failed(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::MODICATIONS_FAILED,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("ERROR APPLYING MODIFICATIONS: {e}")),
            }),
        )
    }


    pub fn err_docker_login(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::DOCKER_LOGIN_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("ERROR CONNECTING TO DOCKER: {e}")),
            }),
        )
    }
    pub fn err_k8s_login(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::K8SCLI_LOGIN_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("ERROR CONNECTING TO K8S: {e}")),
            }),
        )
    }

    pub fn err_docker_del(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::DOCKER_IMG_DEL_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("ERROR DELETING DOCKER IMAGE: {e}")),
            }),
        )
    }
    pub fn err_k8s_del(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::K8S_SERVICE_DEPLOY_DEL_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("ERROR DELETING K8S RESOURCES: {e}")),
            }),
        )
    }


    pub fn unknown_ise(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::UNKNOWN_ISE,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("INTERNAL SERVER ERROR: {e}")),
            }),
        )
    }
    pub fn io_err(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::IO_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("IO ERROR: {e}")),
            }),
        )
    }
    pub fn git_err(meta: Metadata, e: impl Display) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::GIT_ERR,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: Some(format!("GIT ERROR: {e}")),
            }),
        )
    }

}
