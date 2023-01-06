use dotenv::dotenv;
use shiplift::container::ContainerInfo;
use shiplift::image::{ImageBuildChunk};
use std::borrow::Borrow;
use std::fs::{self, read_dir};
use std::env;
use std::io::{Error as IOError};
use std::collections::HashSet;

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

pub async fn docker_login() -> Result<Docker, String> {
    let docker = Docker::new();
    match docker.version().await {
        Ok(_ver) => {
            info!("Successfully connected to docker daemon");
            Ok(docker)
        },
        Err(err) => {
            error!("{}", err);
            if err.to_string().contains("error trying to connect: No such file or directory") {
                warn!("Ensure Docker is running");
            }
            
            Err(err.to_string())
        }, 
    }
    
}

/// Retrieves all Docker images on the system
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

/// Builds a Docker image from the Dockerfile contained in the folder with **challname** (assumes Dockerfile is in the root of the challenge folder provided).
/// Takes in a Vec<&str> of challenge names to support building multiple images via one call.
/// If a challenge already exists, Docker deals with rebuilding and whatnot.
pub async fn build_image(docker: &Docker, list_chall_names : Vec<&str>) -> Result<(), String> {
    let challenge_folder = &get_env("CHALL_FOLDER")?;
    let registry_url = &get_env("DOCKER_REGISTRY_URL")?;

    'chall_to_build: for chall_name in list_chall_names{
        info!("Creating image for: {:?}", chall_name);
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
            
        let mut stream = docker.images().build(&build_options);
        while let Some(build_result) = stream.next().await {
            match build_result {
                // TODO -> Probably want to use resultbuffer for this
                Ok(output) => {
                    match output.borrow() {
                        ImageBuildChunk::Update {stream} => {
                            trace!("{:?}", stream);
                        },
                        ImageBuildChunk::Error {error, ..} => {
                            error!("Error building {:?}", chall_name);
                            debug!("Trace: {:?}", error);
                            warn!("Skipping challenge {:?}, check logs for details...", chall_name);
                            continue 'chall_to_build;
                        }, 
                        ImageBuildChunk::Digest {aux} => {
                            info!("Image digest: {:?}", aux);
                        }
                        // currently not formatting anything with pull status (i.e. pulling ubuntu image for binex challs)
                        // TODO --> Add nice formatting and process the data nicely
                        ImageBuildChunk::PullStatus { .. } => {
                            trace!("{:?}", output);
                        }
                    }
                },
                Err(e) => {
                    error!("Error building docker image");
                    debug!("Docker image build error: {:?}", e);
                    continue 'chall_to_build;
                },
            }
        }
        
        info!("{:?} image has been built", chall_name);
    }

    Ok(())
}

pub async fn retrieve_containers(docker: &Docker) -> Result < Vec<ContainerInfo>, String > {
    match docker.containers().list(&Default::default()).await {
        Ok(containers) => {
            Ok(containers)
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

// TODO --> add support for admin bot challenges (or challs w multiple dockerfiles) (most likely admin bot connection will be dealt w/ k8s side)
// also probably try and figure out a better way of doing this
/// Fetches the name of all folders in the provided **CHALL_FOLDER** env var that contain a Dockerfile in the root of the folder (will be expanded in the future to check recursively for child folders)
pub fn fetch_chall_folder_names() -> Result<Vec<String>, String> {
    let challenge_folder = &get_env("CHALL_FOLDER")?;
    
    let mut local_repo_path = PathBuf::new();
    local_repo_path.push(challenge_folder);

    let mut chall_names : Vec<String> = Vec::new();
    match read_dir(&local_repo_path) {
        Ok(local_repo) => {
            for entry in local_repo {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        error!("Error reading directory");
                        debug!("Trace: {}", err);
                        return Err("Error reading directory".to_owned());
                    }
                };
                
                let path = entry.path();

                if path.is_dir()  {
                    let container_chall_folder = match path.read_dir() {
                        Ok(container_chall_folder) => container_chall_folder,
                        Err(err) => {
                            error!("Error reading directory");
                            debug!("Trace: {}", err);
                            return Err("Error reading directory".to_owned());
                        }
                    };

                    for sub_entry in container_chall_folder {
                        let sub_entry = match sub_entry {
                            Ok(sub_entry) => sub_entry,
                            Err(err) => {
                                error!("Error traversing directory");
                                debug!("Trace: {}", err);
                                return Err("Error traversing directory".to_owned());
                            }
                        };

                        let pathname = sub_entry.file_name();
                        // TODO --> add case for admin bot stuff here, most likely will end up having specific naming conventions for admin bot challs

                        if pathname.eq("Dockerfile") {
                            let chall_name = match path.file_name() {
                                None => {
                                    error!("Error reading challenge name");
                                    debug!("Reading folder name returned None...Ensure pathnames do not end in '/' or '..'");
                                    return Err("Error reading challenge name".to_owned());
                                },
                                Some(name) => {
                                    name.to_string_lossy().to_string()
                                }
                            };

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
            debug!("Trace: {}", err);
            debug!("Path provided: {}", local_repo_path.to_string_lossy());
            return Err(err.to_string());
        }
    }
}

/// Quality of life function --> Builds all images that it finds in the challenge directory
pub async fn build_all_images(docker : &Docker) -> Result<String, String> {
    match fetch_chall_folder_names() {
        Ok(names) => {
            let available_challs_to_deploy : Vec<&str> = names.iter().map(AsRef::as_ref).collect();
            info!("Attempting to build all challenges...");
            build_image(&docker, available_challs_to_deploy).await?;
            info!("Successfully built all images.");
            return Ok("Successfully built all images.".to_string());
        },
        Err(err) => {
            error!("Error fetching folder names");
            debug!("Trace: {}", err);
            return Err(err.to_string());
        }
    };
}

/// Pushes image to remote registry specified by **DOCKER_REGISTRY_URL** env var
/// 
/// Important Note: Does not accurately throw errors/warn if something happens when pushing containers.
// TODO --> Write own push function that impl stream to accurately return errors
pub async fn push_image(docker: &Docker, name: &str) -> Result<(), String> {
    let registry_username = &get_env("DOCKER_REGISTRY_USERNAME")?;
    let registry_password = &get_env("DOCKER_REGISTRY_PASSWORD")?;
    let registry_url = &get_env("DOCKER_REGISTRY_URL")?;

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url.clone())
        .build();

    let mut complete_url = PathBuf::new();
    complete_url.push(registry_url);
    complete_url.push(name);

    info!("Pushing image: {}...", name);
    
    // Unfortunately, working off of a fork of shiplift to get this to work --> push does not impl stream so have to deal with less data for pushing containers
    match docker.images().push(&complete_url.to_string_lossy(), &PushOptions::builder().auth(auth).build()).await {
        Ok(stream) => {
            stream
        },
        Err(e) => {
            error!("Error pushing image");
            debug!("Trace: {:?}", e);
            return Err(e.to_string());
        },
    };

    info!("Pushed image: {}", name);
    warn!("Success may not be accurate. Check registry to ensure status.");
    Ok(())
}

pub async fn pull_image(docker: &Docker, name: &str) -> Result<(), String>{
    let registry_username = &get_env("DOCKER_REGISTRY_USERNAME")?;
    let registry_password = &get_env("DOCKER_REGISTRY_PASSWORD")?;
    let registry_url = &get_env("DOCKER_REGISTRY_URL")?;

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();
        
    let mut complete_url = PathBuf::new();
    complete_url.push(registry_url);
    complete_url.push(name);

    info!("Attempting to pull image: {}", name);

    // TODO --> add better logging for pulling challenges (deal with stream and use resultbuffer to process info)
    let mut stream = docker.images().pull(&PullOptions::builder().auth(auth).image(complete_url.to_string_lossy()).build());
    while let Some(data) = stream.next().await {
        match data {
            Ok(output) => {
                match output.borrow() {
                    ImageBuildChunk::Update {stream} => {
                        trace!("{:?}", stream);
                    },
                    ImageBuildChunk::Error {error, ..} => {
                        error!("Error building {:?}", name);
                        debug!("Trace: {:?}", error);
                        warn!("Skipping challenge {:?}, check logs for details...", name);
                    }, 
                    ImageBuildChunk::Digest {aux} => {
                        info!("Image digest: {:?}", aux);
                    }
                    // currently not formatting anything with pull status (i.e. pulling ubuntu image for binex challs)
                    // TODO --> Add nice formatting and process the data nicely
                    ImageBuildChunk::PullStatus { .. } => {
                        trace!("{:?}", output);
                    }
                }
            },
            Err(e) => {
                error!("Error pulling image");
                debug!("Trace: {:?}", e);
                return Err(e.to_string());
            },
        };
    }

    info!("Successfully pulled image: {}", name);
    Ok(())
}

/// Helper function to just simplify and clean up environment var fetching
/// May want to create custom error types for this to improve error handling
fn get_env(env_name: &str) -> Result<String, String> {
    match env::var(env_name) {
        Ok(val) => Ok(val.to_string()),
        Err(e) => {
            error!("Error reading \"{}\" env var", env_name);
            debug!("Trace: {:?}", e);
            return Err(e.to_string());
        }
    }
}