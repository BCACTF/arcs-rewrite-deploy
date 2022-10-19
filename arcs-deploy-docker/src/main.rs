use dotenv::dotenv;

// use bollard::Docker;

use shiplift::Docker;
use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_docker::{ ResultBuffer, VerifyEnvError };


use std::io::{Result as IOResult, Error as IOError};

extern crate dotenv;

// #[allow(unused_imports)]
// use crate::logging::{ trace, debug, info, warn, error };

#[cfg(unix)]

use arcs_deploy_docker::{logging, build_all_images, build_image, docker_login, fetch_chall_folder_names, push_image, pull_image, retrieve_images, retrieve_containers};

#[tokio::main]
async fn main() -> IOResult<()> {
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;

    dotenv().ok();
    arcs_deploy_docker::verify_env().map_err(to_io_error)?;

    let docker: Docker = docker_login().await;
    // println!("{:?}", retrieve_images(&docker).await);
    // build_image(&docker, vec!["real-deal-html"]).await;
    // push_image(&docker, "real-deal-html").await;

    pull_image(&docker, "real-deal-html").await;
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