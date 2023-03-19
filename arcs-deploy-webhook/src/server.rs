pub mod emitter;
pub mod receiver;
pub mod responses;

use responses::{ Response, Metadata };
use actix_web::{ post, App, HttpServer, web, Responder };
use arcs_deploy_docker::docker_login;
use arcs_deploy_k8s::create_client;
use kube::Client;
// use arcs_deploy_k8s::delete_challenge;
use serde::Deserialize;
use shiplift::Docker;

use crate::receiver::{ delete_challenge, spawn_deploy_req };

use crate::logging::*;
use crate::polling::{ PollingId, poll_deployment };

#[derive(Deserialize)]
pub struct Deploy {
    _type : String,
    deploy_identifier: PollingId,
    chall_name: String,

    chall_meta: Option<String>
}

async fn generate_clients(meta: Metadata) -> Result<(Docker, Client), Response> {
    let docker: Docker = match docker_login().await {
        Ok(docker) => docker,
        Err(err) => return Err(Response::docker_login_err(&err, meta)),
    };
    
    let k8s : Client = match create_client().await {
        Ok(client) => client,
        Err(err) => return Err(Response::k8s_login_err(&err, meta)),
    };

    Ok((docker, k8s))
}

#[post("/")]
pub async fn incoming_post(info: web::Json<Deploy>) -> impl Responder {
    let meta: Metadata = From::from(&info.0);
    
    info!("{} request received", meta.endpoint_name());

    match meta.endpoint_name().as_str() {
        "REDEPLOY" | "DEPLOY" => {
            // Calls to this will be to redeploy/deploy a specific challenge
            let (docker, k8s) = match generate_clients(meta.clone()).await {
                Ok((d, k)) => (d, k),
                Err(resp) => return resp.wrap(),
            };

            match spawn_deploy_req(docker, k8s, meta) {
                Ok(resp) => resp,
                Err(resp) => resp,
            }.wrap()
        },
        "DELETE" => {
            let (docker, k8s) = match generate_clients(meta.clone()).await {
                Ok((d, k)) => (d, k),
                Err(resp) => return resp.wrap(),
            };
            
            delete_challenge(docker, k8s, meta).await.wrap()
        },
        "POLL" => {
            match poll_deployment(info.deploy_identifier) {
                Ok(info) => Response::success(meta, serde_json::to_value(info).ok()),
                Err(poll_id) => Response::poll_id_doesnt_exist(poll_id, meta),
            }.wrap()
        },
        _ => {
            warn!("Endpoint {} not implemented on deploy server", info._type);
            Response::endpoint_err(&info._type, meta).wrap()
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