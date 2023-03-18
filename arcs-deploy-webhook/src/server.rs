pub mod emitter;
pub mod receiver;

use actix_web::{ post, App, HttpServer, web, Responder };
use arcs_deploy_docker::docker_login;
use arcs_deploy_k8s::create_client;
use kube::Client;
// use arcs_deploy_k8s::delete_challenge;
use serde::{ Serialize, Deserialize };
use shiplift::Docker;

use uuid::Uuid;

use crate::receiver::{ delete_challenge, spawn_deploy_req };

use crate::logging::*;

// TODO --> update all challenge_ids, commit_id, racelockid to be UUIDs,
//          parse everything into correct datatypes (everything is just a string right now)
// TODO --> figure out how to get logging to work when a function in a different crate is called
//          most likely want to start server, and then just have all of these functions called asynchronously as things run 
#[derive(Deserialize)]
pub struct Deploy {
    _type : String,
    chall_id: Uuid,
    deploy_race_lock_id: Uuid,
    chall_name: String,
    chall_desc: Option<String>,
    chall_points: Option<String>,
    chall_meta: Option<String>
}

#[derive(Debug, Clone, Serialize)]
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

    let uppercase_type = info._type.to_uppercase();

    match uppercase_type.as_str() {
        "REDEPLOY" | "DEPLOY" => {
            // Calls to this will be to redeploy/deploy a specific challenge
            info!("{} request received", uppercase_type);
            let web::Json(Deploy { chall_name, chall_id, deploy_race_lock_id, ..}) = info;
            match spawn_deploy_req(docker, k8s, chall_name, chall_id, deploy_race_lock_id) {
                Ok(polling_id) => web::Json(Response { status: "Deployment started successfully".into(), message: polling_id.to_string() }),
                Err(resp) => web::Json(resp)
            }
        },
        "DELETE" => {
            info!("{} request received", uppercase_type);
            delete_challenge(docker, k8s, &info.chall_name).await
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