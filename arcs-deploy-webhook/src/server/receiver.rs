use arcs_deploy_docker::{ build_image, docker_login };
use shiplift::Docker;
use actix_web::{ web };

use crate::server::Response;

pub async fn deploy_challenge(name: &String) -> web::Json<Response> {
    println!("Deploying {}...", name);
    let docker: Docker = match docker_login().await {
        Ok(docker) => docker,
        Err(e) => return web::Json(Response{status: "Error logging into docker".to_string(), message: format!("Failed to deploy: {}", e)})
    };

    let build_response = match build_image(&docker, vec![name.as_str()]).await {
        Ok(_) => Response{status: "Success deploying".to_string(), message: "Deployed".to_string()},
        Err(e) => Response{status: "Error deploying".to_string(), message: format!("Failed to deploy: {}", e)}
    };

    println!("Deployed {}", name);
    web::Json(build_response)
}