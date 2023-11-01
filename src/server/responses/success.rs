use arcs_yaml_parser::YamlShape;

use crate::server::utils::api_types::outgoing::{ DeploymentStatus, FromDeploy, Status };

use super::{Metadata, Response, StatusCode};


impl Response {
    pub fn success_deploy_start(meta: Metadata) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        let (status, status_time) = meta.status.into();
        Self(
            StatusCode::ACCEPTED,
            FromDeploy::Status(DeploymentStatus { chall_name, poll_id, status, status_time, err_msg: None }),
        )
    }

    pub fn success_deploy_poll(meta: Metadata, status: crate::polling::DeploymentStatus) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        let (status, status_time) = status.into();
        Self(
            StatusCode::SUCCESS,
            FromDeploy::Status(DeploymentStatus { chall_name, poll_id, status, status_time, err_msg: None }),
        )
    }

    pub fn success_remove(meta: Metadata) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::SUCCESS,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: None,
            }),
        )
    }

    pub fn success_modify_meta(metadata: Metadata, yaml: YamlShape) -> Self {
        let chall_name = Some(yaml.chall_name().to_string());
        let poll_id = metadata.poll_id();
        Self(
            StatusCode::SUCCESS,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: None,
            }),
        )
    }
    pub fn conflict_modify_meta(meta: Metadata) -> Self {
        let chall_name = Some(meta.chall_name().to_string());
        let poll_id = meta.poll_id();
        Self(
            StatusCode::SUCCESS,
            FromDeploy::Status(DeploymentStatus {
                chall_name,
                poll_id,
                status: Status::Unknown,
                status_time: std::time::Duration::ZERO.into(),
                err_msg: None,
            }),
        )
    }

    pub fn success_list_challs(challs: &[&str]) -> Self {
        Self(
            StatusCode::SUCCESS,
            FromDeploy::ChallNameList(challs.iter().copied().map(str::to_string).collect())
        )
    }
}
