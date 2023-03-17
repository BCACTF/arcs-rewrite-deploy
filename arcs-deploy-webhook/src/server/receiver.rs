use arcs_deploy_docker::{ build_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge};

use kube::{Client};
use shiplift::Docker;
use actix_web::{ web };

use crate::server::Response;
use crate::logging::*;

// TODO --> Add function to deploy **everything**, 
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
pub async fn deploy_challenge(docker: Docker, k8s: Client, name: &String, chall_folder_path: &str) -> web::Json<Response> {
    info!("Deploying {} to Kubernetes cluster...", name);

    let status = pull_challenge(docker, name).await;
    if status.status == "Error pulling" { return status; };
    
    let deploy_response = match create_full_k8s_deployment(k8s, vec![name], chall_folder_path).await {
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

// TODO -- Delete docker image from remote registry and local
pub async fn delete_challenge(_docker: Docker, client: Client, name: &String) -> web::Json<Response> {
    warn!("Deleting {}...", name);

    let delete_k8s_response = match delete_k8s_challenge(client, vec![name.as_str()]).await {
        Ok(_) => "Success deleting k8s deployment/service".to_string(),
        Err(e) => format!("Error deleting k8s deployment/service: {:?}", e)
    };

    if delete_k8s_response.contains("Error") {
        return web::Json(Response{status: "Error deleting k8s deployment/service".to_string(), message: delete_k8s_response});
    }

    println!("Deleted '{name}'");
    web::Json(Response{status: "Success deleting".to_string(), message: format!("Deleted '{name}'")})
}