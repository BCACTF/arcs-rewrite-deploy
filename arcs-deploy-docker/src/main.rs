use dotenv::dotenv;

use bollard::Docker;

use std::default::Default;

use arcs_deploy_logging::{set_up_logging, DEFAULT_LOGGGING_TARGETS};
use arcs_deploy_docker::{ ResultBuffer, VerifyEnvError };


use std::io::{Result as IOResult, Error as IOError};

use futures::stream::StreamExt;

extern crate dotenv;

// #[allow(unused_imports)]
// use crate::logging::{ trace, debug, info, warn, error };

#[cfg(unix)]

use arcs_deploy_docker::{logging, build_image, retrieve_images, retrieve_containers, docker_login};

#[tokio::main]
async fn main() -> IOResult<()> {
    set_up_logging(&DEFAULT_LOGGGING_TARGETS, logging::DEFAULT_TARGET_NAME)?;

    dotenv().ok();
    arcs_deploy_docker::verify_env().map_err(to_io_error)?;

    let docker: Docker = docker_login().await;
    let images = retrieve_images(&docker).await.unwrap();    
    let containers = retrieve_containers(&docker).await.unwrap();

    images
        .iter()
        .for_each(|image| {
            println!("{} : {:?}", image.id, image.repo_tags);
        });

    containers
        .iter()
        .for_each(|container| {
            println!("{:?} : {:?}", container.id, container.names);
        });

    // get a list of the subdirectories in the chall files directory, and from there you can access everything else

    build_image(&docker, "real-deal-html").await;
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