use dotenv::dotenv;
use std::fs;
use std::env;
use std::io::Error as IOError;
use std::collections::HashSet;

mod result_buffer;
pub use result_buffer::ResultBuffer;

use bollard::service::{ContainerSummary, ImageSummary};

use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::image::{ListImagesOptions, BuildImageOptions};

use std::default::Default;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Builder;

use futures::stream::StreamExt;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub enum VerifyEnvError {
    VerifyFailed(Vec<String>),
    IOError(IOError),
}

impl From<IOError> for VerifyEnvError {
    fn from(err: IOError) -> Self {
        VerifyEnvError::IOError(err)
    }
}

pub async fn docker_login() -> Docker {
    #[allow(unused_variables)]
    // look into switching connect_with_local_defaults to connect_with_socket_defaults
    let docker = match Docker::connect_with_local_defaults() {
        Ok(docker) => {
            info!("Successfully connected to Docker Daemon");
            docker   
        },
        Err(err) => {
            error!("Error connecting to Docker. Ensure daemon is running");
            info!("Connection Error: {}", err);
            todo!("handle error");
        }
    };
    docker
}

pub async fn retrieve_images(docker: &Docker) -> Result< Vec<ImageSummary>, String > {

    let images = match docker.list_images(Some(ListImagesOptions::<String> {
        all: true,
        ..Default::default()
    })).await {
        Ok(images) => {
            info!("Docker images successfully fetched");
            images
        }, 
        Err(err) => {
            error!("Error fetching Docker images");
            info!("Image Error: {}", err);
            return Err("".to_owned())
        }
    };
    Ok(images.to_vec())
}

pub async fn build_image(docker: &Docker, chall_name : &str){
    // todo - error handling
    let tar_path = tar_chall(chall_name).await;
    // let tar_path = match tar_chall(chall_name) {
    //     Ok(tar_path) => {
    //         info!("Successfully tarred {}", chall_name);
    //         tar_path
    //     },
    //     Err(err) => {
    //         error!("Error tarring {}", chall_name);
    //         info!("Trace : {}", err);
    //     }
    // }; 

    let options = BuildImageOptions {
        dockerfile: "Dockerfile",
        t: chall_name,
        rm: true,
        ..Default::default()
    };

    let mut chall = File::open(&tar_path).unwrap();
    let mut contents = Vec::new();
    chall.read_to_end(&mut contents).unwrap();

    let mut s = docker.build_image(options, None, Some(contents.into()));
    
    let mut result_buffer = ResultBuffer::new();

    while let Some(build_info_image_result) = s.next().await {
        match build_info_image_result {
            Ok(new_info) => result_buffer.process_build_info(new_info),
            Err(err) => {
                error!("Building docker image failed!");
                info!("Docker image build error: {:?}", err);
                break;
            },
        };
    };
}

pub async fn retrieve_containers(docker: &Docker) -> Result < Vec<ContainerSummary>, String > {
    #[allow(unused_variables)]
    let containers = match docker.list_containers(Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()

    })).await {
        Ok(containers) => {
            info!("Docker containers successfully fetched");
            containers
        },
        Err(err) => {
            error!("Error fetching Docker containers");
            info!("Container Error: {}", err);
            return Err(err.to_string());
        }
    };
    Ok(containers.to_vec())
}

pub async fn tar_chall(chall_name : &str) -> PathBuf {
    // add error handling here

    let tar_path = {
        let mut tar_path = PathBuf::new();
        tar_path.push(r"./tarball_challs/");
        tar_path.push(chall_name);
        tar_path.set_extension("tar");
        tar_path
    };

    let file = File::create(&tar_path).unwrap();
    let mut tarball_builder = Builder::new(file);
    let chall_src_path: PathBuf = [&env::var("CHALL_FOLDER").unwrap(), chall_name].into_iter().collect();
    tarball_builder.append_dir_all(".", &chall_src_path).unwrap();
    tarball_builder.finish().unwrap();
    tar_path
}

pub fn verify_env() -> Result<(), VerifyEnvError> {
    use logging::*;

    dotenv().ok();
    let mut missing_env_vars: Vec<String> = Vec::new();

    let req_envs_string = fs::read_to_string(".required_envs")?;
    let req_envs: Vec<&str> = req_envs_string.lines().collect();

    let vars: Vec<String> = env::vars().map(|(var_name, _)| var_name).collect();

    let existing_envs: HashSet<&str> = vars.iter().map(String::as_str).collect();

    for env in req_envs {
        if !existing_envs.contains(env) {
            error!("Missing required environment variable: {}", env);
            missing_env_vars.push(env.to_string());
        }
    }
    if !missing_env_vars.is_empty() {
        Err(VerifyEnvError::VerifyFailed(missing_env_vars))
    } else {
        info!("Environment variables verified");
        Ok(())
    }
}