pub mod emitter;
pub mod receiver;
pub mod responses;

use responses::{ Response, Metadata };
use actix_web::{ App, HttpServer, web, Responder };
use arcs_yaml_editor::Modifications;

use actix_web::post;
use arcs_deploy_docker::docker_login;
use arcs_deploy_k8s::create_client;
use kube::Client;
use serde::Deserialize;
use shiplift::Docker;

use crate::auth::validate_auth_token;
use crate::receiver::{ delete_challenge, spawn_deploy_req, update_yaml };
use crate::emitter::sync_metadata_with_webhook;

use crate::logging::*;
use crate::polling::PollingId;

use crate::env::{port, deploy_address};

use actix_web_httpauth::middleware::HttpAuthentication;

/// Struct that represents incoming post requests to the Deploy server
/// 
/// Every deploy struct uniquely identifies a deploy request that is being made to the server
/// 
/// ## Fields
/// - `_type` - The type of request that is being made
/// - `deploy_identifier` - The identifier of the deployment that is being made, formatted as: 
/// ```
///     chall_id.deploy_id
///            OR 
///     {
///     'chall_name': Uuid,
///     'deploy_id': Uuid
///     }
/// ```
/// - `chall_name` - The name of the challenge that is being deployed
#[derive(Deserialize)]
pub struct Deploy {
    __type : String,
    deploy_identifier: PollingId,
    chall_name: String,
    modifications: Option<Modifications>,
}

/// Generates a Docker and K8s client for use in the deploy server
/// ## Returns
/// - `Ok((Docker, Client))` - If both clients were successfully generated, with `Docker` being DockerClient and `Client` being K8sClient
/// - `Err(Response)` - If either client failed to be generated
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

/// The main entry point for the deploy server
/// 
/// All incoming requests pass through this, which calls the corresponding functions depending on the request type
/// 
/// ## Current Endpoints
/// - `REDEPLOY` | `Deploy` - Fully deploys a challenge, or redeploys a challenge if it already exists
/// - `DELETE` - Deletes a challenge from the cluster and removes local Docker image
/// - `POLL` - Polls the status of a deployment
/// 
/// ## Returns
///  - `actix_web::web::Json<Response>` - Returns a `actix_web::web::JSON` object returned by the endpoint that was requested. This JSON object ultimately gets sent out as a request response.
#[post("/")]
async fn incoming_post(info: web::Json<Deploy>) -> impl Responder {
    let meta: Metadata = From::from(&info.0);
    
    info!("{} request received", meta.endpoint_name());

    match meta.endpoint_name().as_str() {
        "REDEPLOY" | "DEPLOY" => {
            let (docker, k8s) = match generate_clients(meta.clone()).await {
                Ok((d, k)) => (d, k),
                Err(resp) => return resp.wrap(),
            };

            // spawns a Tokio task to handle the deployment of challenge, allows multiple requests to be handled at once
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
            
            delete_challenge(&docker, &k8s, meta).await.wrap()
        },
        "POLL" => {
            let metadata = Metadata::from(&info.0);
            if metadata.status_is_unknown() {
                Response::poll_id_doesnt_exist(info.0.deploy_identifier, metadata)
            } else {
                Response::success(metadata, None)
            }.wrap()
        },
        "MODIFY_META" => {
            let meta = Metadata::from(&info.0);

            let Some(modifications) = info.0.modifications else {
                return Response::modifications_missing(meta).wrap();
            };

            let new_yaml = match update_yaml(meta.chall_name(), modifications, &meta).await {
                Ok(new_yaml) => new_yaml,
                Err(resp) => return resp,
            };

            sync_metadata_with_webhook(&meta, new_yaml).await.wrap()
        }
        _ => {
            warn!("Endpoint {} not implemented on deploy server", info.__type);
            Response::endpoint_err(&info.__type, meta).wrap()
        },
    }
}

// TODO - migrate bind to environment variables
pub async fn initialize_server() -> std::io::Result<()> {
    let server_ip = deploy_address().strip_prefix("http://").or(deploy_address().strip_prefix("https://")).unwrap();
    let server_port : u16 = port().parse().unwrap();

    info!("Deploy server listening on {}:{}", server_ip, server_port);

    HttpServer::new(|| {
        let auth = HttpAuthentication::bearer(validate_auth_token);
        App::new()
        .wrap(auth)
        .service(incoming_post)
    })
    .bind((server_ip, server_port))?
    .run()
    .await
}