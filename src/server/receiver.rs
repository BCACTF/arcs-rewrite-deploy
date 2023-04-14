use arcs_deploy_docker::{ build_image, delete_image as delete_docker_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge, get_chall_folder};

use arcs_yaml_parser::{File, YamlVerifyError};
use kube::Client;
use shiplift::Docker;
use super::responses::{ Metadata, Response };
use serde_json::json;

use crate::emitter::{send_deployment_success, send_deployment_failure};
use crate::logging::*;
use crate::polling::{ PollingId, register_chall_deployment, fail_deployment, succeed_deployment, advance_deployment_step };

use arcs_deploy_static::deploy_static_files;

#[derive(Debug, Clone)]
pub struct BuildChallengeErr(String);


/// Enum that represents the different errors that can occur during the deploy process
/// 
/// ## Variants
/// - `FileUpload` - Error uploading file(s) to CDN
/// - `Build` - Error building Docker image
/// - `Push` - Error pushing to remote Docker registry
/// - `Pull` - Error pulling from remote Docker registry
/// - `Fetch` - Error fetching local challenge folder
/// - `Deploy` - Error deploying to Kubernetes cluster
#[derive(Debug, Clone)]
pub enum DeployProcessErr {
    FileUpload(Vec<File>),
    Build(String),
    Push(String),
    Pull(String),
    Fetch(String),
    Deploy(String),
}

impl From<(DeployProcessErr, Metadata)> for Response {
    fn from((err, meta): (DeployProcessErr, Metadata)) -> Self {
        use DeployProcessErr::*;
        match err {
            FileUpload(files) => Response::server_deploy_process_err(
                0,
                "Error uploading file(s) to CDN",
                Some(json!({ "files": files })),
                meta,
            ),
            Build(s) => Response::server_deploy_process_err(
                1,
                "Error building docker image",
                Some(json!({ "reason": s })),
                meta,
            ),
            Push(s) => Response::server_deploy_process_err(
                2,
                "Error pushing to registry",
                Some(json!({ "reason": s })),
                meta,
            ),
            Pull(s) => Response::server_deploy_process_err(
                3,
                "Error pulling from registry",
                Some(json!({ "reason": s })),
                meta,
            ),
            Fetch(s) => Response::server_deploy_process_err(
                4,
                "Error fetching challenge folder",
                Some(json!({ "reason": s })),
                meta,
            ),
            Deploy(s) => Response::server_deploy_process_err(
                5,
                "Error deploying to Kubernetes",
                Some(json!({ "reason": s })),
                meta,
            ),
        }
    }
}


// TODO --> Add function to deploy everything, 
// initial deployments to k8s clusters & general instance management
// (this may be done through ansible but setting up cluster as well)

pub async fn build_challenge(docker: Docker, name: &String, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting build; name: {name} poll_id: {polling_id}");
    build_image(&docker, vec![name.as_str()]).await.map_err(DeployProcessErr::Build)
}

pub async fn push_challenge(docker: Docker, name: &String, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting push; name: {name} poll_id: {polling_id}");
    push_image(&docker, name).await.map_err(DeployProcessErr::Push)
}

pub async fn pull_challenge(docker: Docker, name: &String, polling_id: PollingId) -> Result<(), DeployProcessErr> {
    info!("Starting pull; name: {name} poll_id: {polling_id}");
    pull_image(&docker, name).await.map_err(DeployProcessErr::Pull)
}

// may want to move the other two functions into this one and just call this when user asks for deploy/redeploy
// response message is port challenge is running on (or if it's not running, No Port Returned)

pub async fn deploy_challenge(
    docker: Docker,
    k8s: Client,
    name: &String,
    chall_folder_path: Option<&str>,
    polling_id: PollingId,
) -> Result<Vec<i32>, DeployProcessErr> {
    info!("Deploying {} to Kubernetes cluster...", name);

    let chall_folder = get_chall_folder(chall_folder_path);

    pull_challenge(docker, name, polling_id).await?;
    
    match create_full_k8s_deployment(&k8s, vec![name], Some(&chall_folder)).await {
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
pub async fn delete_challenge(docker: Docker, client: Client, meta: Metadata) -> Response {
    let name = meta.chall_name();
    
    warn!("Deleting {}...", name);

    // TODO: Use the variables! (better logs please)
    match delete_k8s_challenge(&client, vec![name.as_str()]).await {
        Ok(_) => {
            info!("Successfully deleted {} from Kubernetes cluster", name);
            "Success deleting Kubernetes deployment/service".to_string()
        },
        Err(e) => {
            error!("Error deleting {} from Kubernetes cluster", name);
            error!("Trace: {}", e);
            return Response::k8s_service_deploy_del_err(e, meta); 
        } 
    };
    // TODO --> add delete docker container so things delete properly

    // TODO: Use the variables! (better logs please)
    #[allow(unused_variables)]
    match delete_docker_image(&docker, name).await {
        Ok(v) => {
            info!("Successfully deleted {} from Docker", name);
            "Success deleting Docker image".to_string()
        },
        Err(e) => {
            return Response::docker_img_del_err(e, meta);
        } 
    };

    println!("Deleted '{name}'");
    let name = name.clone();
    Response::success(meta, Some(json!({ "chall_name": name })))
}




/// Convenience function that calls `advance_deployment_step` on an ongoing deployment and logs the result.
pub fn advance_with_fail_log(polling_id: PollingId) -> bool {
    match advance_deployment_step(polling_id, None) {
        Ok(new_step) => {
            info!("Deployment step advanced to `{}` for {polling_id}", new_step.get_str());
            true
        }
        Err(e) => {
            error!("Failed to advance deployment step for {polling_id} (KILLED): {e:?}");
            false
        }
    }
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
    let name = meta.chall_name().clone();


    if let Err(status) = register_chall_deployment(polling_id) {
        return Err(Response::poll_id_already_in_use(polling_id, status, meta));
    }

    let spawn_meta = meta.clone();
    tokio::spawn(async move {
        let meta = spawn_meta;

        use arcs_deploy_static::fetch_chall_yaml;
        let chall_yaml = fetch_chall_yaml(meta.chall_name().as_str());


        let chall_yaml = if let Some(chall_yaml) = chall_yaml {
            match chall_yaml {
                Ok(yaml) => yaml,
                Err(e) => {
                    error!("Failed to fetch challenge yaml for {} ({}) with err {:?}", meta.chall_name(), polling_id, e);
                    if fail_deployment(polling_id, (e, meta.clone()).into()).is_err() {
                        error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
                    }
                    send_failure_message(&meta, "Fetch Challenge YAML").await;
                    return;
                }
            }
        } else {
            error!("Failed to fetch challenge yaml for {} ({})", meta.chall_name(), polling_id);
            if fail_deployment(polling_id, (YamlVerifyError::OsError, meta.clone()).into()).is_err() {
                error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
            }
            send_failure_message(&meta, "Fetch Challenge YAML").await;
            return;
        };

        
        if chall_yaml.deploy().is_none() {
            if let Err(failed_files) = deploy_static_files(&docker, meta.chall_name().as_str()).await {
                error!("Failed to deploy static files {:?} for {} ({})", failed_files, meta.chall_name(), polling_id);
                if fail_deployment(polling_id, (DeployProcessErr::FileUpload(failed_files), meta.clone()).into()).is_err() {
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

            return;
        }

        // DOCKER CHALLENGES BUILD STARTING FROM HERE, STATIC CHALLS ALREADY RETURNED
        if let Err(build_err) = build_challenge(docker.clone(), &name, polling_id).await {
            error!("Failed to build `{name}` ({polling_id}) with err {build_err:?}");
            if fail_deployment(polling_id, (build_err, meta.clone()).into()).is_err() {
                error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
            }
            send_failure_message(&meta, "Build").await;
            return;
        }
        if !advance_with_fail_log(polling_id) { return; }
        
    
        if let Err(push_err) = push_challenge(docker.clone(), &name, polling_id).await {
            error!("Failed to push `{name}` ({polling_id}) with err {push_err:?}");
            if fail_deployment(polling_id, (push_err, meta.clone()).into()).is_err() {
                error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
            }
            send_failure_message(&meta, "Push").await;
            return;
        }
        if !advance_with_fail_log(polling_id) { return; }

       let ports = match deploy_challenge(docker.clone(), client.clone(), &name, None, polling_id).await {
            Ok(ports) => {
                info!("Successfully deployed `{name}` ({polling_id}) to port(s): {:?}", &ports);
                if succeed_deployment(polling_id, &ports).is_err() {
                    error!("`succeed_deployment` failed to mark polling id {polling_id} as succeeded");
                }
                ports
            },
            Err(deploy_err) => {
                error!("Failed to deploy `{name}` ({polling_id}) with err {deploy_err:?}");
                if fail_deployment(polling_id, (deploy_err, meta.clone()).into()).is_err() {
                    error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
                }
                send_failure_message(&meta, "Deploy").await;
                return;
            }
        };

        if let Err(failed_files) = deploy_static_files(&docker, meta.chall_name().as_str()).await {
            error!("Failed to deploy static files {:?} for {} ({})", failed_files, meta.chall_name(), polling_id);
            if fail_deployment(polling_id, (DeployProcessErr::FileUpload(failed_files), meta.clone()).into()).is_err() {
                error!("`fail_deployment` failed to mark polling id {polling_id} as errored");
            }
            send_failure_message(&meta, "Deploy Static Files").await;
            return;
        }

        // TODO --> on a failed to parse file path or other yaml error here, send out a deploy failure message (or try to at least)
        match send_deployment_success(&meta, Some(ports)).await {
            Ok(_) => info!("Successfully sent deployment success message for {} ({})", meta.chall_name(), polling_id),
            Err(e) => error!("Failed to send deployment success message for {} ({}): {e:?}", meta.chall_name(), polling_id),
        };
    });

    Ok(Response::success(
        meta,
        Some(json!({
            "status": "Deployment started successfully", 
            "message": polling_id,
        })),
    ))
}

async fn send_failure_message(meta: &Metadata, message: &str) {
    match send_deployment_failure(meta, format!("Failed to deploy {}: {} Error", meta.chall_name(), message)).await {
        Ok(_) => info!("Successfully sent deployment failure message for {} ({})", meta.chall_name(), meta.poll_id()),
        Err(e) => error!("Failed to send deployment failure message for {} ({}): {e:?}", meta.chall_name(), meta.poll_id()),
    };
}