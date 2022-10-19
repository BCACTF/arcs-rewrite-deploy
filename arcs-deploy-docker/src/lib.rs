use dotenv::dotenv;
use shiplift::container::ContainerInfo;
use std::fs::{self, read_dir};
use std::env;
use std::io::{Error as IOError};
use std::collections::HashSet;

// TODO - UNCOMMENT ONCE FIXED
// mod result_buffer;
// pub use result_buffer::ResultBuffer;

use shiplift::{Docker, image::{PushOptions, PullOptions, BuildOptions, ImageInfo}};

use std::default::Default;
use std::path::{PathBuf};

use futures::stream::StreamExt;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub mod result_buffer;
pub use result_buffer::ResultBuffer;

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
    let docker = Docker::new();
    match docker.version().await {
        Ok(ver) => info!("Successfully connected to docker daemon..."),
        Err(err) => error!("Error: {}", err), 
    }
    docker
}

pub async fn retrieve_images(docker: &Docker) -> Result<Vec<ImageInfo>, String> {
    match docker.images().list(&Default::default()).await {
        Ok(images) => {
            Ok(images)
        },
        Err(e) => {
            error!("Error occurred when retrieving images... {:?}", e);
            Err(e.to_string())
        },
    }
}

pub async fn build_image(docker: &Docker, list_chall_names : Vec<&str>){
    for chall_name in list_chall_names{
        let challenge_folder = env::var("CHALL_FOLDER").unwrap().to_string();
        let registry_url = env::var("DOCKER_REGISTRY_URL").unwrap().to_string();


        info!("Creating image for : {:?}", chall_name);
        let mut challenge_path = PathBuf::new();
        challenge_path.push(challenge_folder);
        challenge_path.push(chall_name);

        let mut full_registry_path = PathBuf::new();
        full_registry_path.push(registry_url);
        full_registry_path.push(chall_name);

        let build_options = BuildOptions::builder(challenge_path.to_string_lossy().to_string())
            .tag(full_registry_path.to_string_lossy())
            .dockerfile("Dockerfile")
            .rm(true)
            .build();
            
        // let mut result_buffer = <ResultBuffer>::new().with_progress_logging(stdout());
        let mut stream = docker.images().build(&build_options);
        while let Some(build_result) = stream.next().await {
            match build_result {
                // Ok(output) => result_buffer.process_build_info(output),
                Ok(output) => {
                    info!("{:?}", output);
                },
                Err(e) => {
                    error!("Error building docker image");
                    info!("Docker image build error: {:?}", e);
                    return;
                },
            }
        }
        
        info!("{:?} image has been built", chall_name);
    }
}

pub async fn retrieve_containers(docker: &Docker) -> Result < Vec<ContainerInfo>, String > {
    match docker.containers().list(&Default::default()).await {
        Ok(info) => {
            Ok(info)
        },
        Err(e) => {
            error!("Error occurred when retrieving containers... {:?}", e);
            Err(e.to_string())
        },
    }
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

// TODO --> add support for admin bot challenges (or challs w multiple dockerfiles) and figure out how to return them/display them (maybe docker-compose?)
// also probably try and figure out a better way of doing this
pub fn fetch_chall_folder_names() -> Result<Vec<String>, String> {
    let mut local_repo_path = PathBuf::new();
    local_repo_path.push(&env::var("CHALL_FOLDER").unwrap());
    let mut chall_names : Vec<String> = Vec::new();
    match read_dir(&local_repo_path) {
        Ok(local_repo) => {
            for entry in local_repo {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        error!("Error reading directory");
                        info!("Trace: {}", err);
                        return Err("Error reading directory".to_owned());
                    }
                };
                
                let path = entry.path();

                if path.is_dir()  {
                    let container_chall_folder = match path.read_dir() {
                        Ok(container_chall_folder) => container_chall_folder,
                        Err(err) => {
                            error!("Error reading directory");
                            info!("Trace: {}", err);
                            return Err("Error reading directory".to_owned());
                        }
                    };
                    for sub_entry in container_chall_folder {
                        let sub_entry = match sub_entry {
                            Ok(sub_entry) => sub_entry,
                            Err(err) => {
                                error!("Error traversing directory");
                                info!("Trace: {}", err);
                                return Err("Error traversing directory".to_owned());
                            }
                        };
                        let pathname = sub_entry.file_name();
                        if pathname.eq("Dockerfile") {
                            // may need error handling here for unwrap, but can't see this being an issue
                            let chall_name = path.file_name().unwrap().to_str().unwrap().to_string();
                            info!("Found chall: {:?}", chall_name);
                            chall_names.push(chall_name);
                        }
                    };
                }
            }
            Ok(chall_names)
        },
        Err(err) => {
            error!("Error fetching challenge folder directory");
            info!("Trace: {}", err);
            info!("Path provided: {}", local_repo_path.to_str().unwrap());
            return Err(err.to_string());
        }
    }
}

pub async fn build_all_images(docker : &Docker) -> Result<String, String> {
    info!("Attempting to build all challenges...");
    match fetch_chall_folder_names() {
        Ok(names) => {
            let available_challs_to_deploy : Vec<&str> = names.iter().map(AsRef::as_ref).collect();
            build_image(&docker, available_challs_to_deploy).await;
            info!("All images created");
            return Ok("Successfully built all images.".to_string());
        },
        Err(err) => {
            error!("Error fetching folder names");
            info!("Trace: {}", err);
            return Err(err.to_string());
        }
    };
}

pub async fn push_image(docker: &Docker, name: &str) {
    let registry_username = &env::var("DOCKER_REGISTRY_USERNAME").unwrap();
    let registry_password = &env::var("DOCKER_REGISTRY_PASSWORD").unwrap();
    let registry_url = &env::var("DOCKER_REGISTRY_URL").unwrap();
    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();
    let mut complete_url = PathBuf::new();
    complete_url.push(registry_url);
    complete_url.push(name);

    info!("Pushing image: {}", name);
    let stream = match docker.images().push(&complete_url.to_string_lossy(), &PushOptions::builder().auth(auth).build()).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Error pushing image");
            info!("Trace: {:?}", e);
            return;
        },
    };
    info!("Successfully pushed image: {}", name);
}

pub async fn pull_image(docker: &Docker, name: &str) {
    let registry_username = &env::var("DOCKER_REGISTRY_USERNAME").unwrap();
    let registry_password = &env::var("DOCKER_REGISTRY_PASSWORD").unwrap();
    let registry_url = &env::var("DOCKER_REGISTRY_URL").unwrap();

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();
    let mut complete_url = PathBuf::new();
    complete_url.push(registry_url);
    complete_url.push(name);

    info!("Attempting to pull image: {}", name);
    let mut stream = docker.images().pull(&PullOptions::builder().auth(auth).image(complete_url.to_string_lossy()).build());
    while let Some(data) = stream.next().await {
        let data = match data {
            // probably want to use a result buffer for this in the future
            Ok(data) => info!("{:?}", data),
            Err(e) => {
                error!("Error pulling image");
                info!("Trace: {:?}", e);
                return;
            },
        };
    }

    info!("Successfully pulled image: {}", name);
}