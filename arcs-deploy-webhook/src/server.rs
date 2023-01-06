pub mod emitter;
pub mod receiver;

use actix_web::{ post, App, HttpServer, web, Responder };
use serde::{ Serialize, Deserialize };

use crate::receiver::deploy_challenge;

// TODO --> update all challenge_ids, commit_id, racelockid to be UUIDs,
//          parse everything into correct datatypes (everything is just a string right now)
// TODO --> figure out how to get logging to work when a function in a different crate is called
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
    match info._type.as_str() {
        "redeploy" => {
            println!("redeploy");
            match &info.chall_name {
                Some(chall_name) => {
                    deploy_challenge(chall_name).await
                },
                None => web::Json(Response{status: "Error deploying".to_string(), message: "Chall name not specified".to_string()})
            }
        },
        "build" => {
            unimplemented!();
            println!("build");
        },
        _ => {
            unimplemented!();
            println!("other");
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