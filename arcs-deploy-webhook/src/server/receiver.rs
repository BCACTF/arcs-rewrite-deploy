use arcs_deploy_docker::{ build_image, delete_image as delete_docker_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge, get_chall_folder};

use kube::{Client};
use shiplift::Docker;
use actix_web::{ web };
use uuid::Uuid;

use crate::server::Response;
use crate::logging::*;
use crate::polling::{ PollingId, register_chall_deployment, fail_deployment, succeed_deployment, advance_deployment_step };

#[derive(Debug, Clone)]
pub struct BuildChallengeErr(String);


#[derive(Debug, Clone)]
pub enum DeployProcessErr {
    Build(String),
    Push(String),
    Pull(String),
    Fetch(String),
    Deploy(String),
}

impl From<DeployProcessErr> for Response {
    fn from(err: DeployProcessErr) -> Self {
        use DeployProcessErr::*;
        match err {
            Build(s) => Response { status: "Error building".to_string(), message: format!("Failed to build: {}", s) },
            Push(s) => Response { status: "Error pushing".to_string(), message: format!("Failed to push: {}", s) },
            Pull(s) => Response { status: "Error pulling".to_string(), message: format!("Failed to pull: {}", s) },
            Fetch(s) => Response { status: "Error fetching".to_string(), message: format!("Failed to fetch: {}", s) },
            Deploy(s) => Response { status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", s) },
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

    let chall_folder = get_chall_folder(chall_folder_path).map_err(DeployProcessErr::Fetch)?;

    pull_challenge(docker, name, polling_id).await?;
    
    match create_full_k8s_deployment(k8s, vec![name], Some(&chall_folder)).await {
        Ok(ports) => {
            if ports.len() <= 0 { 
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

pub async fn delete_challenge(docker: Docker, client: Client, name: &String) -> web::Json<Response> {
    warn!("Deleting {}...", name);

    match delete_k8s_challenge(client, vec![name.as_str()]).await {
        Ok(_) => {
            info!("Successfully deleted {} from k8s cluster", name);
            "Success deleting k8s deployment/service".to_string()
        },
        Err(e) => {
            error!("Error deleting {} from k8s cluster", name);
            error!("Trace: {}", e);
            return web::Json(Response{status: "Error deleting k8s deployment/service".to_string(), message: e});
        } 
    };

    match delete_docker_image(&docker, name).await {
        Ok(_) => {
            info!("Successfully deleted {} from Docker", name);
            "Success deleting docker image".to_string()
        },
        Err(e) => {
            return web::Json(Response{status: "Error deleting docker image".to_string(), message: e});
        } 
    };

    println!("Deleted '{name}'");
    web::Json(Response{status: "Success deleting".to_string(), message: format!("Deleted '{name}'")})
}





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

pub fn spawn_deploy_req(docker: Docker, client: Client, name: String, chall_id: Uuid, race_lock: Uuid) -> Result<PollingId, Response> {
    let polling_id = PollingId::new(chall_id, race_lock);

    if let Err(e) = register_chall_deployment(polling_id) {
        return Err(Response {
            status: "Error registering deployment".to_string(),
            message: format!("{e:?}")
        });
    }

    tokio::spawn(async move {
        if let Err(build_err) = build_challenge(docker.clone(), &name, polling_id).await {
            error!("Failed to build `{name}` ({polling_id}) with err {build_err:?}");
            fail_deployment(polling_id, build_err.into());
            return;
        }
        if !advance_with_fail_log(polling_id) { return; }
        
    
        if let Err(push_err) = push_challenge(docker.clone(), &name, polling_id).await {
            error!("Failed to push `{name}` ({polling_id}) with err {push_err:?}");
            fail_deployment(polling_id, push_err.into());
            return;
        }
        if !advance_with_fail_log(polling_id) { return; }

       match deploy_challenge(docker.clone(), client.clone(), &name, None, polling_id).await {
            Ok(response) => {
                info!("Successfully deployed `{name}` ({polling_id}) to port(s): {response:?}");
                succeed_deployment(polling_id, response);
            },
            Err(deploy_err) => {
                error!("Failed to deploy `{name}` ({polling_id}) with err {deploy_err:?}");
                fail_deployment(polling_id, deploy_err.into());
                return;
            }
        }
    });

    Ok(polling_id)
}

