pub mod emitter;
pub mod receiver;
pub mod responses;

use lazy_static::lazy_static;

use actix_web::dev::ServiceRequest;
use responses::{ Response, Metadata };
use actix_web::{ post, App, HttpServer, web, Responder };
use arcs_deploy_docker::docker_login;
use arcs_deploy_k8s::create_client;
use kube::Client;
use serde::Deserialize;
use shiplift::Docker;

use crate::receiver::{ delete_challenge, spawn_deploy_req };

use crate::logging::*;
use crate::polling::{ PollingId, poll_deployment };

use constant_time_eq::{ constant_time_eq_32 };
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config, Error};
use actix_web_httpauth::extractors::AuthenticationError;
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
    _type : String,
    deploy_identifier: PollingId,
    chall_name: String,
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

lazy_static! {
    static ref WEBHOOK_SERVER_TOKEN: String = std::env::var("WEBHOOK_SERVER_AUTH_TOKEN").expect("WEBHOOK_SERVER_TOKEN must be set");
    // parsed into a [u8;32] for constant time comparison
    static ref WEBHOOKARR : [u8;32]= match (&WEBHOOK_SERVER_TOKEN.as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting from slice to [u8;32]");
            error!("{:?}", e);
            panic!("Failed to convert WEBHOOK_SERVER_AUTH_TOKEN to [u8;32]");
        },
    };
}

// todo - potentially switch over to jwt? or hmac?
/// Function to validate the authentication token of a request
/// 
/// Reads in from the `Authentication` header of the request
/// 
/// ## Returns
/// - `Ok(ServiceRequest)` - If the token is valid
/// - `Err((actix_web::Error, ServiceRequest))` - If the token is invalid : short circuits request and returns status to client
async fn validate_auth_token (
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {

    let config = req
        .app_data::<Config>()
        .map(|data| data.clone())
        .unwrap_or_else(Default::default);

    let credarr : [u8;32]= match (&credentials.token().as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting from slice to [u8;32]");
            error!("{:?}", e);
            return Err((AuthenticationError::from(config).with_error(Error::InvalidToken).into(), req))
        },
    };

    if constant_time_eq_32(&credarr, &WEBHOOKARR) {
        return Ok(req);
    }

    error!("Unauthenticated request received");
    Err((AuthenticationError::from(config).with_error(Error::InvalidToken).into(), req))
}

pub async fn initialize_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        let auth = HttpAuthentication::bearer(validate_auth_token);
        App::new()
        .wrap(auth)
        .service(incoming_post)
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}