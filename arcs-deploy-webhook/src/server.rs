pub mod emitter;
pub mod receiver;

use actix_web::{ post, App, HttpServer, web, Responder };
use arcs_deploy_docker::docker_login;
use arcs_deploy_k8s::create_client;
use kube::Client;
// use arcs_deploy_k8s::delete_challenge;
use serde::{ Serialize, Deserialize };
use shiplift::Docker;

use crate::receiver::{ build_challenge, delete_challenge, push_challenge, deploy_challenge };

use crate::logging::*;

// TODO --> update all challenge_ids, commit_id, racelockid to be UUIDs,
//          parse everything into correct datatypes (everything is just a string right now)
// TODO --> figure out how to get logging to work when a function in a different crate is called
//          most likely want to start server, and then just have all of these functions called asynchronously as things run 
#[derive(Deserialize)]
pub struct Deploy {
    _type : String,
    chall_id: String,
    chall_name: Option<String>,
    // commit_id: Option<u32>,
    deploy_race_lock_id: Option<String>,
    chall_desc: Option<String>,
    chall_points: Option<String>,
    chall_meta: Option<String>
}

#[derive(Serialize)]
pub struct Response {
    status: String,
    message: String
}

#[post("/")]
pub async fn incoming_post(info: web::Json<Deploy>) -> impl Responder {
    info!("RECIEVED POST REQUEST");

    let docker: Docker = match docker_login().await {
        Ok(docker) => docker,
        Err(e) => return web::Json(Response{status: "Error logging into docker".to_string(), message: format!("Failed to deploy: {}", e)})
    };
    
    let k8s : Client = match create_client().await {
        Ok(client) => client,
        Err(e) => return web::Json(Response{status: "Error creating k8s client".to_string(), message: format!("Failed to delete: {}", e)})
    };

    match info._type.as_str() {
        "redeploy" | "deploy" => {
            // Calls to this will be to redeploy/deploy a specific challenge
            info!("{} request received", info._type.to_uppercase());
            match &info.chall_name {
                Some(chall_name) => {
                    let build_status: actix_web::web::Json<Response> = build_challenge(docker.clone(), chall_name).await;
                    if build_status.status == "Error building" { return build_status };

                    let push_status: actix_web::web::Json<Response> = push_challenge(docker.clone(), chall_name).await;
                    if push_status.status == "Error pushing" { return push_status };

                    // may need to do some stuff for admin bots here
                    let deploy_status: actix_web::web::Json<Response> = deploy_challenge(docker.clone(), k8s.clone(), chall_name, None).await;
                    deploy_status
                },
                None => web::Json(Response{status: "Error deploying".to_string(), message: "Chall name not specified".to_string()})
            }
        },
        "delete" => {
            info!("{} request received", info._type.to_uppercase());
            match &info.chall_name {
                Some(chall_name) => {
                    delete_challenge(docker, k8s, chall_name).await
                },
                None => web::Json(Response{status: "Error deleting".to_string(), message: "Chall name not specified".to_string()})
            }
        },
        _ => {
            info!("{} request received", info._type);
            warn!("Endpoint {} not implemented on deploy server", info._type);
            web::Json(Response{status: "Endpoint Not Implemented".to_string(), message: format!("Endpoint {} not implemented on deploy server", info._type)})
        },
    }
}

pub async fn initialize_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(incoming_post)
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}