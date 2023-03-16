use arcs_deploy_docker::{ build_image, push_image, pull_image };
use arcs_deploy_k8s::{ create_challenge as create_full_k8s_deployment, delete_challenge as delete_k8s_challenge};

use kube::{Client};
use shiplift::Docker;
use actix_web::{ web };

use crate::server::Response;

// TODO --> Add function to deploy **everything**, 
// initial deployments to k8s clusters & general instance management
// (this may be done through ansible but setting up cluster as well)

//  TODO --> build k8s instance using this as well
pub async fn build_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    println!("Building '{}'...", name);

    let build_response = match build_image(&docker, vec![name.as_str()]).await {
        Ok(_) => Response{status: "Success deploying".to_string(), message: "Deployed".to_string()},
        Err(e) => Response{status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", e)}
    };

    println!("Successfully built '{}'", name);
    web::Json(build_response)
}

pub async fn push_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    println!("Pushing '{name}' to remote registry...");

    let push_response = match push_image(&docker, name).await {
        Ok(_) => Response{status: "Success pushing".to_string(), message: format!("Pushed {name}")},
        Err(e) => {
            println!("Error pushing: {}", e);
            Response{status: "Error pushing".to_string(), message: format!("Failed to push: {}", e)}
        }
    };

    println!("Pushed '{name}'... WARN: CHECK REMOTE FOR STATUS");
    web::Json(push_response)
}

pub async fn pull_challenge(docker: Docker, name: &String) -> web::Json<Response> {
    println!("Pulling '{name}' from remote registry");

    let pullresponse = match pull_image(&docker, name).await {
        Ok(_) => Response{status: "Success pulling".to_string(), message: format!("Pulled '{name}'")},
        Err(e) => {
            println!("Error pulling: {}", e);
            Response{status: "Error pulling".to_string(), message: format!("Failed to pull: {}", e)}
        }
    };

    println!("Pulled '{name}' from registry");
    web::Json(pullresponse)
}

// may want to move the other two functions into this one and just call this when user asks for deploy/redeploy
pub async fn deploy_challenge(docker: Docker, k8s: Client, name: &String, chall_folder_path: &str) -> web::Json<Response> {
    println!("Beginning deployment of '{name}' to k8s cluster...");

    println!(">>> Pulling image...");
    let status = pull_challenge(docker, name).await;
    if status.status == "Error pulling" { return status; };
    println!(">>> Successfully pulled image...");

    let deploy_response = match create_full_k8s_deployment(k8s, vec![name], chall_folder_path).await {
        Ok(_) => Response{status: "Success deploying".to_string(), message: "Deployed".to_string()},
        Err(e) => Response{status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", e)}
    };

    println!("Successfully deployed '{name}' to k8s cluster");
    web::Json(deploy_response)
}

// TODO -- Delete docker image from remote registry and local
pub async fn delete_challenge(_docker: Docker, client: Client, name: &String) -> web::Json<Response> {
    println!("Deleting '{name}'...");

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