pub mod container_links;
pub mod discord_message;
pub mod links;


use crate::logging::*;
use crate::server::responses::Metadata;

use arcs_static::fetch_chall_yaml;


pub async fn get_yaml_shape(meta: &Metadata) -> Result<yaml::YamlShape, String> {
    match fetch_chall_yaml(meta.chall_name()).await {
        Some(Ok(yaml)) => Ok(yaml),
        Some(Err(e)) => {
            error!("Failed to parse chall.yaml for {}: {:#?}", meta.chall_name(), e);
            Err("Failed to parse chall.yaml for challenge".to_string())
        },
        None => {
            error!("Failed to find chall.yaml for challenge: {}", meta.chall_name());
            Err("Failed to find chall.yaml for challenge".to_string())
        },
    }
}
