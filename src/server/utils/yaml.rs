use tokio::fs::{ read_to_string, write };

use arcs_deploy_static::{chall_yaml_path, fetch_chall_yaml};
use arcs_yaml_editor::Modifications;
use arcs_yaml_parser::YamlShape;

use crate::server::responses::{Response, Metadata};
use crate::logging::*;


pub async fn update_yaml_file(chall_folder_name: &str, modifications: Modifications, meta: &Metadata) -> Result<YamlShape, Response> {
    let meta = meta.clone();

    let yaml_location = chall_yaml_path(chall_folder_name);
    let Ok(old_yaml) = read_to_string(&yaml_location).await else {
        error!("Failed to read chall.yaml for {} @ {:?}", meta.chall_name(), yaml_location);
        return Err(Response::chall_doesnt_exist(chall_folder_name, meta));
    };

    if let Some(new_yaml) = modifications.apply(&old_yaml) {
        if new_yaml == old_yaml {
            warn!("Yaml unchanged by modifications");
            // return Err(Response::modifications_failed(meta));
        }
    }

    match modifications.apply(&old_yaml) {
        Some(new_yaml) => if let Err(e) = write(&yaml_location, new_yaml).await {
            error!("Failed to write new chall.yaml");
            return Err(Response::ise(&e.to_string(), meta));
        },
        None => {
            error!("Failed to apply modifications to chall.yaml");
            return Err(Response::modifications_failed(meta))
        },
    };

    let new_yaml = match fetch_chall_yaml(chall_folder_name) {
        Some(Ok(new_yaml)) => new_yaml,
        Some(Err(e)) => {
            debug!("Yaml error: {e}");
            if std::fs::write(&yaml_location, old_yaml).is_ok() {
                error!("Invalid chall YAML created! Rolled back successfully. THIS IS SOMETHING TO BE LOOKED INTO.");
                return Err(Response::ise("Invalid chall YAML created! Reverted.", meta));
            } else {
                error!("Invalid chall YAML created! Failed to roll back. THIS IS A CRITICAL ERROR!");
                return Err(Response::ise("Invalid chall YAML created! Failed to roll back.", meta));
            }
        },
        None => {
            if std::fs::write(&yaml_location, old_yaml).is_ok() {
                error!("Couldn't find chall.yaml! Rolled back successfully. THIS IS SOMETHING TO BE LOOKED INTO.");
                return Err(Response::ise("Couldn't find chall.yaml! Reverted.", meta));
            } else {
                error!("Couldn't find chall.yaml! Failed to roll back. THIS IS A CRITICAL ERROR!");
                return Err(Response::ise("Couldn't find chall.yaml! Failed to roll back.", meta));
            }
        },
    };

    Ok(new_yaml)
}

pub async fn handle_yaml_get(meta: &Metadata) -> Option<YamlShape> {
    use crate::polling::fail_deployment;
    use crate::server::utils::state_management::send_failure_message;
    use arcs_yaml_parser::YamlVerifyError;

    let meta = meta.clone();
    let polling_id = meta.poll_id();

    let chall_yaml = fetch_chall_yaml(meta.chall_name().as_str());

    if let Some(chall_yaml) = chall_yaml {
        match chall_yaml {
            Ok(yaml) => Some(yaml),
            Err(e) => {
                error!("Failed to fetch challenge yaml for {} ({}) with err {:?}", meta.chall_name(), polling_id, e);
                if fail_deployment(polling_id, e.to_string()).is_err() {
                    error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
                }
                send_failure_message(&meta, "Fetch Challenge YAML").await;
                None
            }
        }
    } else {
        error!("Failed to fetch challenge yaml for {} ({})", meta.chall_name(), polling_id);
        if fail_deployment(polling_id, YamlVerifyError::OsError.to_string()).is_err() {
            error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
        }
        send_failure_message(&meta, "Fetch Challenge YAML").await;
        None
    }
}
