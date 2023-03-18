use arcs_deploy_docker::{ build_image, delete_image as delete_docker_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge, get_chall_folder};

use kube::{Client};
use shiplift::Docker;
use actix_web::{ web };

use crate::server::Response;
use crate::logging::*;

// TODO --> Add function to deploy everything, 
// initial deployments to k8s clusters & general instance management
// (this may be done through ansible but setting up cluster as well)

//  TODO --> build k8s instance using this as well
pub async fn build_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    let build_response = match build_image(&docker, vec![name.as_str()]).await {
        Ok(_) => Response{status: "Success deploying".to_string(), message: "Deployed".to_string()},
        Err(e) => Response{status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", e)}
    };

    web::Json(build_response)
}

pub async fn push_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    let push_response = match push_image(&docker, name).await {
        Ok(_) => Response{status: "Success pushing".to_string(), message: format!("Pushed {name}")},
        Err(e) => Response{status: "Error pushing".to_string(), message: format!("Failed to push: {}", e)}
    };

    web::Json(push_response)
}

pub async fn pull_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    let pullresponse = match pull_image(&docker, name).await {
        Ok(_) => Response{status: "Success pulling".to_string(), message: format!("Pulled '{name}'")},
        Err(e) => Response{status: "Error pulling".to_string(), message: format!("Failed to pull: {}", e)}
    };

    web::Json(pullresponse)
}

// may want to move the other two functions into this one and just call this when user asks for deploy/redeploy
// response message is port challenge is running on (or if it's not running, No Port Returned)
pub async fn deploy_challenge(docker: Docker, k8s: Client, name: &String, chall_folder_path: Option<&str>) -> web::Json<Response> {
    info!("Deploying {} to Kubernetes cluster...", name);

    let chall_folder = match get_chall_folder(chall_folder_path) {
        Ok(path) => path,
        Err(e) => return web::Json(Response{status: "Error fetching challenge folder path".to_string(), message: format!("Failed to deploy: {}", e)})
    };

    let status = pull_challenge(docker, name).await;
    if status.status == "Error pulling" { return status; };
    
    let deploy_response = match create_full_k8s_deployment(k8s, vec![name], Some(&chall_folder)).await {
        Ok(ports) => {
            if ports.len() <= 0 { 
                error!("Error deploying {} to k8s cluster", name);
                debug!("No Port Returned");
                Response{status: "Error deploying".to_string(), message: "No Port Returned".to_string()}
            } else {
                Response{status: "Success deploying".to_string(), message: format!("{:?}", ports)}
            }
        }
        Err(e) => Response{status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", e)}
    };

    info!("Successfully deployed {} to k8s cluster", name);
    web::Json(deploy_response)
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