use tokio::fs::{ read_to_string, write };

use arcs_static::{chall_yaml_path, fetch_chall_yaml};
use yaml_editor::Modifications;
use yaml::YamlShape;

use crate::server::responses::{Response, Metadata};
use crate::logging::*;


pub async fn update_yaml_file(chall_folder_name: &str, modifications: Modifications, meta: &Metadata) -> Result<YamlShape, Response> {
    let meta = meta.clone();

    let yaml_location = chall_yaml_path(chall_folder_name);
    let Ok(old_yaml) = read_to_string(&yaml_location).await else {
        error!("Failed to read chall.yaml for {} @ {:?}", meta.chall_name(), yaml_location);
        return Err(Response::err_chall_name_doesnt_exist(meta, chall_folder_name));
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
            return Err(Response::git_err(meta, format!("Failed to write new chall.yaml: {:#?}", e)));
        },
        None => {
            error!("Failed to apply modifications to chall.yaml");
            return Err(Response::err_modifications_failed(meta, "Failed to apply modifications to chall.yaml"));
        },
    };

    let new_yaml = match fetch_chall_yaml(chall_folder_name).await {
        Some(Ok(new_yaml)) => new_yaml,
        Some(Err(e)) => {
            debug!("Yaml error: {e}");
            if std::fs::write(&yaml_location, old_yaml).is_ok() {
                error!("Invalid chall YAML created! Rolled back successfully. THIS IS SOMETHING TO BE LOOKED INTO.");
                return Err(Response::err_modifications_failed(meta, "Invalid chall YAML created! Reverted."));
            } else {
                error!("Invalid chall YAML created! Failed to roll back. THIS IS A CRITICAL ERROR!");
                return Err(Response::err_modifications_failed(meta, "Invalid chall YAML created! Failed to roll back."));
            }
        },
        None => {
            if std::fs::write(&yaml_location, old_yaml).is_ok() {
                error!("Couldn't find chall.yaml! Rolled back successfully. THIS IS SOMETHING TO BE LOOKED INTO.");
                return Err(Response::io_err(meta, "Couldn't find chall.yaml! Reverted."));
            } else {
                error!("Couldn't find chall.yaml! Failed to roll back. THIS IS A CRITICAL ERROR!");
                return Err(Response::io_err(meta, "Couldn't find chall.yaml! Failed to roll back."));
            }
        },
    };

    Ok(new_yaml)
}

pub async fn handle_yaml_get(meta: &Metadata) -> Option<YamlShape> {
    use crate::polling::fail_deployment;
    use crate::server::utils::state_management::send_failure_message;
    use yaml::YamlVerifyError;

    let meta = meta.clone();
    let polling_id = meta.poll_id();

    let chall_yaml = fetch_chall_yaml(meta.chall_name().as_str()).await;

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
