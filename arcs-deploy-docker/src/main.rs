use shiplift::Docker;
use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_docker::{ VerifyEnvError };

use std::io::{Result as IOResult, Error as IOError};

extern crate dotenv;
use dotenv::dotenv;

#[cfg(unix)]

use arcs_deploy_docker::{logging, build_image, fetch_chall_folder_names, docker_login, retrieve_images, build_all_images, push_image, pull_image};

#[tokio::main]
async fn main() -> IOResult<()> {
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;

    dotenv().ok();
    arcs_deploy_docker::verify_env().map_err(to_io_error)?;

    let docker: Docker = match docker_login().await {
        Ok(docker) => docker,
        Err(e) => return Err(IOError::new(std::io::ErrorKind::Other, e)),
    };
    
    // println!("{:#?}", retrieve_images(&docker).await);
    // println!("{:?}", fetch_chall_folder_names());
    // build_all_images(&docker).await.unwrap();
    build_image(&docker, vec!["agent-rocket"]).await;
    push_image(&docker, "agent-rocket").await;
    // match pull_image(&docker, "real-deal-html").await {
    //     Err(e) => return  Err(IOError::new(std::io::ErrorKind::Other, e)),
    //     _ => (),
    // };

    Ok(())
}

fn to_io_error(error: VerifyEnvError) -> IOError {
    match error {
        VerifyEnvError::IOError(error) => error,
        VerifyEnvError::VerifyFailed(missed_envs) => IOError::new(
            std::io::ErrorKind::Other,
            format!("Missing variable: {:?}", missed_envs),
        ),
    }
}