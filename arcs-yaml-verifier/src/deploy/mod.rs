pub mod error;
pub mod structs;



use serde_yaml::Value as YamlValue;

use crate::{structs::{get_type}, files::Flop};

use self::{
    error::{DeployOptionsError, DeploymentTargetOptionsError, ExposeError},
    structs::{DeployOptions, DeployTarget, NetworkProtocol},
};


const DEFAULT_REPLICAS: u8 = 1;

pub fn parse_expose(expose: &str) -> Result<NetworkProtocol, ExposeError> {

    let (port, protocol) = expose
        .split_once('/')
        .ok_or_else(|| ExposeError::BadFormat(expose.to_string()))?;

    let (port, protocol_is_tcp) = (
        port.parse::<u32>(),
        match protocol {
            "udp" => Ok(false),
            "tcp" => Ok(true),
            _ => Err(()),
        },
    );

    match (port, protocol_is_tcp) {
        (Ok(port), Ok(protocol_is_tcp)) => Ok(if protocol_is_tcp {
            NetworkProtocol::Tcp(port)
        } else {
            NetworkProtocol::Udp(port)
        }),
        (port, protocol_is_tcp) => Err(ExposeError::BadParts {
            data: expose.to_string(),
            port: port.is_err(),
            protocol: protocol_is_tcp.is_err(),
        }),
    }
}

pub fn parse_deploy_target(value: &YamlValue) -> Result<DeployTarget, DeploymentTargetOptionsError> {
    let mapping = value.as_mapping().ok_or_else(|| DeploymentTargetOptionsError::BadBaseType(get_type(value)))?;


    let expose = mapping
        .get("expose")
        .map(YamlValue::as_str).flatten()
        .map_or(Err(ExposeError::Missing), parse_expose);


    let replicas = if let Some(replicas_val) = mapping.get("replicas") {
        Some(
            replicas_val
                .as_u64()
                .map(u8::try_from)
                .map(Result::ok).flatten()
                .ok_or_else(|| get_type(replicas_val))
        )
    } else { None }.flop();

    match (expose, replicas) {
        (Ok(expose), Ok(replicas)) => Ok(DeployTarget {
            expose,
            replicas: replicas.unwrap_or(DEFAULT_REPLICAS),
        }),
        (expose, replicas) => Err(DeploymentTargetOptionsError::Parts {
            expose: expose.err(),
            replicas_invalid: replicas.err(),
        })
    }

}

pub fn parse_deploy(value: &YamlValue) -> Result<DeployOptions, DeployOptionsError> {
    let mapping = value.as_mapping().ok_or_else(|| DeployOptionsError::BadBaseType(get_type(value)))?;

    let web = mapping
        .get("web")
        .map(parse_deploy_target)
        .flop();

    let admin = mapping
        .get("admin")
        .map(parse_deploy_target)
        .flop();

    let nc = mapping
        .get("nc")
        .map(parse_deploy_target)
        .flop();

    match (web, admin, nc) {
        (Ok(web), Ok(admin), Ok(nc)) => Ok(DeployOptions { web, admin, nc }),
        (web, admin, nc) => Err(DeployOptionsError::Parts {
            web: web.err(),
            admin: admin.err(),
            nc: nc.err(),
        })
    }
}

