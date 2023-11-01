use std::path::Path;

use arcs_deploy_docker::{ build_image, delete_image as delete_docker_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge, get_chall_folder };
use arcs_deploy_static::deploy_static_files;

use arcs_deploy_static::env::chall_folder_default;
use arcs_yaml_editor::Modifications;
use arcs_yaml_parser::{
    deploy::structs::{DeployTarget, DeployTargetType},
    YamlShape
};


use kube::Client;
use shiplift::Docker;
use super::responses::{ Metadata, Response };

use crate::server::utils::{
    errors::DeployProcessErr,
    git::{ ensure_repo_up_to_date, make_commit, push_all },
    state_management::{ advance_with_fail_log, send_failure_message },
    yaml::{ update_yaml_file, handle_yaml_get },
};
use crate::emitter::send_deployment_success;
use crate::logging::*;
use crate::polling::{ PollingId, register_chall_deployment, fail_deployment, succeed_deployment };


#[derive(Debug, Clone)]
pub struct BuildChallengeErr(String);





// TODO --> Add function to deploy everything, 
// initial deployments to k8s clusters & general instance management
// (this may be done through ansible but setting up cluster as well)

pub async fn build_challenge(docker: &Docker, name: &String, inner_path: Option<&Path>, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting build; name: {name} poll_id: {polling_id}");
    build_image(docker, name.as_str(), inner_path).await.map_err(DeployProcessErr::Build)
}

pub async fn push_challenge(docker: &Docker, name: &String, inner_path: Option<&Path>, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting push; name: {name} poll_id: {polling_id}");
    push_image(docker, name, inner_path).await.map_err(DeployProcessErr::Push)
}

pub async fn pull_challenge(docker: &Docker, name: &String, inner_path: Option<&Path>, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting pull; name: {name} poll_id: {polling_id}");
    pull_image(docker, name, inner_path).await.map_err(DeployProcessErr::Pull)
}

// may want to move the other two functions into this one and just call this when user asks for deploy/redeploy
// response message is port challenge is running on (or if it's not running, No Port Returned)

pub async fn deploy_challenge(
    docker: &Docker,
    k8s: &Client,
    name: &String,
    chall_folder_path: Option<&str>,
    inner_path: Option<&Path>,
    polling_id: PollingId,
) -> Result<Vec<i32>, DeployProcessErr> {
    info!("Deploying {} to Kubernetes cluster...", name);

    let chall_folder = get_chall_folder(chall_folder_path);

    pull_challenge(docker, name, inner_path, polling_id).await?;
    
    // FIXME --> Update k8s to use the inner_paths as well
    match create_full_k8s_deployment(k8s, vec![name], Some(&chall_folder)).await {
        Ok(ports) => {
            if ports.is_empty() { 
                error!("Error deploying {} ({polling_id}) to k8s cluster", name);
                error!("No Port Returned");

                Err(DeployProcessErr::Deploy("No Port(s) Returned".into()))
            } else {
                info!("Successfully deployed {name} ({polling_id}) to port(s): {ports:?}");
                Ok(ports)
            }
        }
        Err(s) => {
            error!("Failed to deploy {name} ({polling_id}) to k8s cluster");
            error!("Trace: {}", s);
            Err(DeployProcessErr::Deploy(s))
        }
    }
}

// FIXME: Deprecation bad.
pub async fn delete_challenge(docker: &Docker, client: &Client, meta: Metadata) -> Response {
    let name = meta.chall_name();
    
    warn!("Deleting {}...", name);

    // TODO: Use the variables! (better logs please)
    match delete_k8s_challenge(client, vec![name.as_str()]).await {
        Ok(_) => {
            info!("Successfully deleted {} from Kubernetes cluster", name);
            "Success deleting Kubernetes deployment/service".to_string()
        },
        Err(e) => {
            error!("Error deleting {} from Kubernetes cluster", name);
            error!("Trace: {}", e);
            return Response::err_k8s_del(meta, e); 
        } 
    };
    // TODO --> add delete docker container so things delete properly

    // TODO: Use the variables! (better logs please)
    // FIXME: make this use an actual inner_path
    #[allow(unused_variables)]
    match delete_docker_image(docker, name, None).await {
        Ok(v) => {
            info!("Successfully deleted {} from Docker", name);
            "Success deleting Docker image".to_string()
        },
        Err(e) => {
            error!("Error deleting {} from Docker: {e:?}", name);
            return Response::err_docker_del(meta, e);
        } 
    };

    debug!("Deleted '{name}'");
    Response::success_remove(meta)
}


async fn do_static_deployment(docker: Docker, meta: Metadata) {
    let polling_id = meta.poll_id();

    if let Err(failed_files) = deploy_static_files(&docker, meta.chall_name().as_str()).await {
        error!("Failed to deploy static files {:?} for {} ({})", failed_files, meta.chall_name(), polling_id);
        if fail_deployment(polling_id, DeployProcessErr::FileUpload(failed_files).to_string()).is_err() {
            error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
        }
        send_failure_message(&meta, "Deploy Static Files").await;
        return;
    }

    warn!("No deploy section found in challenge yaml for {} ({})", meta.chall_name(), polling_id);
    // TODO --> Kinda hacky passing in empty slice, fix later probably (please)
    if succeed_deployment(polling_id, &[]).is_err() {
        error!("`succeed_deployment` failed to mark polling id {polling_id} as succeeded");
    }
    
    match send_deployment_success(&meta, None).await {
        Ok(_) => info!("Successfully sent deployment success message for {} ({})", meta.chall_name(), polling_id),
        Err(e) => error!("Failed to send deployment success message for {} ({}): {e:?}", meta.chall_name(), polling_id),
    };
}

async fn deploy_target(
    docker: &Docker,
    client: &Client,
    target: DeployTarget,
    target_type: DeployTargetType,
    meta: &Metadata,
    deployed_servers: &mut Vec<(DeployTargetType, Vec<i32>)>,
) -> bool {
    let meta = meta.clone();
    let polling_id = meta.poll_id();
    let name = meta.chall_name().clone();

    // if built_path defaulted or set to ".", subfolder is None
    let build_path_buf = if target.build.to_string_lossy() == "." {
        None
    } else {
        Some(target.build.clone())
    };

    let build_path = build_path_buf.as_deref(); 

    if let Err(build_err) = build_challenge(docker, &name, build_path, polling_id).await {
        error!("Failed to build `{name}` ({polling_id}) with err {build_err:?}");
        if fail_deployment(polling_id, build_err.to_string()).is_err() {
            error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
        }
        send_failure_message(&meta, "Build").await;
        return false;
    }
    if !advance_with_fail_log(polling_id) { return false; }
    

    if let Err(push_err) = push_challenge(docker, &name, build_path, polling_id).await {
        error!("Failed to push `{name}` ({polling_id}) with err {push_err:?}");
        if fail_deployment(polling_id, push_err.to_string()).is_err() {
            error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
        }
        send_failure_message(&meta, "Push").await;
        return false;
    }
    if !advance_with_fail_log(polling_id) { return false; }

    let ports = match deploy_challenge(docker, client, &name, None, build_path, polling_id).await {
        Ok(ports) => {
            info!("Successfully deployed `{name}` ({polling_id}) to port(s): {:?}", &ports);
            if succeed_deployment(polling_id, &ports).is_err() {
                error!("`succeed_deployment` failed to mark polling id {polling_id} as succeeded");
            }
            ports
        },
        Err(deploy_err) => {
            error!("Failed to deploy `{name}` ({polling_id}) with err {deploy_err:?}");
            if fail_deployment(polling_id, deploy_err.to_string()).is_err() {
                error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
            }
            send_failure_message(&meta, "Deploy").await;
            return false;
        }
    };
    
    // FIXME --> This might break if there are two different deployed containers that have a weird container/image name --> fix will most likely include server type possibly??
    if let Err(failed_files) = deploy_static_files(docker, meta.chall_name().as_str()).await {
        error!("Failed to deploy static files {:?} for {} ({})", failed_files, meta.chall_name(), polling_id);
        if fail_deployment(polling_id, DeployProcessErr::FileUpload(failed_files).to_string()).is_err() {
            error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
        }
        send_failure_message(&meta, "Deploy Static Files").await;
        return false;
    }

    deployed_servers.push((target_type, ports));

    true
}

/// Registers a new deployment with the given polling id provided in `meta`
/// 
/// Spawns a Tokio task to handle the deployment of a challenge
/// 
/// ## Returns
/// - `Ok(Response)` : Deployment was successfully registered, returns success registering message
/// - `Err(Response)` : Deployment was not registered due to an error, error contains trace
pub fn spawn_deploy_req(docker: Docker, client: Client, meta: Metadata) -> Result<Response, Response> {
    let polling_id = meta.poll_id();


    if let Err(status) = register_chall_deployment(polling_id) {
        if !status.is_finished() {
            return Err(Response::poll_id_in_use(meta, polling_id, status));
        }
        let failed_to_update_poll_id = crate::polling::_update_deployment_state(
            polling_id,
            crate::polling::DeploymentStatus::InProgress(
                std::time::Instant::now(),
                crate::polling::DeployStep::Building,
            ),
        ).is_err();
        if failed_to_update_poll_id {
            return Err(Response::unknown_ise(meta, "Failed to update deployment state"));
        }
    }

    let spawn_meta = meta.clone();
    tokio::spawn(async move {
        let meta = spawn_meta;

        let Some(chall_yaml) = handle_yaml_get(&meta).await else { return };
        
        let Some(deploy_options) = chall_yaml.deploy() else {
            do_static_deployment(docker, meta).await;
            return;
        };

        // DOCKER CHALLENGES BUILD STARTING FROM HERE, STATIC CHALLS ALREADY RETURNED
        // to build multiple things iterate over chall.yaml with deploy fields and then you can take the path they say to build and build that path, return the links as a tuple with the type of server built and then from tehre that makes it easier to display and you dont need to rework everything

        let collected = deploy_options.clone()
            .into_iter()
            .collect::<Vec<(DeployTarget, DeployTargetType)>>();

        let mut deployed_servers : Vec<(DeployTargetType, Vec<i32>)> = Vec::new();
        for (target, target_type) in collected {
            if !deploy_target(&docker, &client, target, target_type, &meta, &mut deployed_servers).await {
                return;
            }
        }

        // TODO --> on a failed to parse file path or other yaml error here, send out a deploy failure message (or try to at least)
        match send_deployment_success(&meta, Some(deployed_servers)).await {
            Ok(_) => info!("Successfully sent deployment success message for {} ({})", meta.chall_name(), polling_id),
            Err(e) => error!("Failed to send deployment success message for {} ({}): {e:?}", meta.chall_name(), polling_id),
        };
    });


    use std::time::{ SystemTime, UNIX_EPOCH };
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|duration| duration.as_millis());

    Ok(Response::success_deploy_start(meta))
}

pub async fn update_yaml(chall_folder_name: &str, modifications: Modifications, meta: &Metadata) -> Result<YamlShape, Response> {
    let meta = meta.clone();

    let repo_path = Path::new(chall_folder_default());

    let should_push = ensure_repo_up_to_date(repo_path, &meta)?;
    trace!("Repo up to date");

    let new_yaml = update_yaml_file(chall_folder_name, modifications, &meta).await?;
    debug!("{chall_folder_name}");

    let message = format!("ADMIN_PANEL_MANAGEMENT: updated chall.yaml for challenge `{chall_folder_name}`");
    
    let yaml_location_relative = std::path::PathBuf::from_iter([chall_folder_name, "chall.yaml"].into_iter());
    
    make_commit(repo_path, &[&yaml_location_relative], &message, &meta)?;
    if should_push {
        push_all(repo_path, &meta)?;
    }

    Ok(new_yaml)
}
