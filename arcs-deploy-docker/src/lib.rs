use dotenv::dotenv;
use env::chall_folder_default;
use shiplift::container::ContainerInfo;
use shiplift::image::ImageBuildChunk;
use std::borrow::Borrow;
use std::fs::{self, read_dir};
use std::io::{Error as IOError};
use std::collections::HashSet;

use shiplift::{Docker, image::{PushOptions, PullOptions, BuildOptions, ImageInfo}};

use std::default::Default;
use std::path::{PathBuf};

mod env;
pub use env::check_env_vars;

use futures::stream::StreamExt;
#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

use crate::env::{reg_url, reg_username, reg_password};

pub enum VerifyEnvError {
    VerifyFailed(Vec<String>),
    IOError(IOError),
}

impl From<IOError> for VerifyEnvError {
    fn from(err: IOError) -> Self {
        VerifyEnvError::IOError(err)
    }
}

/// Creates the [`Docker`][Docker] client for use throughout the deployment process
/// 
/// ## Returns
/// - `Ok(Docker)` - Docker client
/// - `Err(String)` - Error trace
pub async fn docker_login() -> Result<Docker, String> {
    let docker = Docker::new();
    match docker.version().await {
        Ok(_ver) => {
            info!("Successfully connected to docker daemon");
            Ok(docker)
        },
        Err(err) => {
            error!("{}", err);
            if err.to_string().contains("error trying to connect") {
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

/// Retrieve all Docker containers on the system
pub async fn retrieve_containers(docker: &Docker) -> Result <Vec<ContainerInfo>, String> {
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

// todo --> fix error propagation, make them return not strings and an actual error type 
/// Builds a Docker image from the Dockerfile contained in the folder with a given `chall_name`
/// 
/// Currently assumes Dockerfile is in the root of the challenge folder provided
/// 
/// If a challenge already exists, Docker deals with rebuilding and whatnot. If an error occurs while building, logs the error and skips to the next challenge.
/// 
/// ## Parameters
/// - `docker` : [`Docker`][Docker]
///     - Docker client to build the challenge with
/// - `list_chall_names` : `Vec<&str>`
///     - List of challenge names to build an image for
/// 
/// ## Returns
/// - `Ok(())` - Image(s) built successfully
/// - `Err(String)` - Error trace
pub async fn build_image(docker: &Docker, list_chall_names : Vec<&str>) -> Result<(), String> {
    let challenge_folder = chall_folder_default();
    let registry_url = reg_url();

    let path_to_registry = PathBuf::from(registry_url);

    'chall_to_build: for chall_name in list_chall_names{
        info!("Creating image for: {:?}", chall_name);
        let mut challenge_path = PathBuf::from(challenge_folder);
        let mut full_registry_path = path_to_registry.clone();
        
        challenge_path.push(chall_name);
        full_registry_path.push(chall_name);

        let build_options = BuildOptions::builder(challenge_path.to_string_lossy().to_string())
            .tag(full_registry_path.to_string_lossy())
            .dockerfile("Dockerfile")
            .rm(true)
            .build();
            
        let mut stream = docker.images().build(&build_options);
        while let Some(build_result) = stream.next().await {
            match build_result {
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

/// Helper function to ensure that all required environment variables are set
/// 
/// Reads in the `.required_envs` file and checks that all of the environment variables listed in that file are set
/// 
/// ## Returns
/// - `Ok(())` - All required environment variables are set
/// - `Err(VerifyEnvError)` - Error indicating that a variable was not set. Logs the list of missing variables
pub fn verify_env() -> Result<(), VerifyEnvError> {
    use logging::*;

    dotenv().ok();
    let mut missing_env_vars: Vec<String> = Vec::new();

    let req_envs_string = fs::read_to_string(".required_envs")?;
    let req_envs: Vec<&str> = req_envs_string.lines().collect();

    let vars: Vec<String> = std::env::vars().map(|(var_name, _)| var_name).collect();

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
/// Fetches the name of all folders in the `CHALL_FOLDER` environment variable that contain a Dockerfile in their root
/// 
/// ## Returns
/// - `Ok(Vec<String>)` - List of challenges with Dockerfiles in root
/// - `Err(String)` - Error trace (likely a missing environment variable or failure to read a directory)
pub fn fetch_chall_folder_names() -> Result<Vec<String>, String> {
    let local_repo_path = PathBuf::from(chall_folder_default());

    let mut chall_names : Vec<String> = Vec::new();
    match read_dir(&local_repo_path) {
        Ok(local_repo) => {
            for entry in local_repo {
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(err) => {
                        error!("Error reading directory");
                        debug!("Trace: {}", err);
                        return Err("Error reading directory".to_owned());
                    }
                };

                if path.is_dir() {
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
                                    warn!("Ensure pathnames do not end in '/' or '..'");
                                    debug!("Reading folder name returned None.");
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

/// Quality of life function that builds all images with Dockerfiles that it finds in the `CHALL_FOLDER` direectory
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

// TODO --> Write own push function that impl stream to better update users on progress
/// Pushes image to remote registry specified by `DOCKER_REGISTRY_URL` env var
/// 
/// Authenticates with the `DOCKER_REGISTRY_USERNAME` and `DOCKER_REGISTRY_PASSWORD` environment variables
/// 
/// ## Returns
/// - `Ok(())` - Image successfully pushed
/// - `Err(String)` - Error occurred while pushing
pub async fn push_image(docker: &Docker, name: &str) -> Result<(), String> {
    let registry_username = reg_username();
    let registry_password = reg_password();
    let registry_url = reg_url();

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url.clone())
        .build();

    let mut complete_url = PathBuf::from(registry_url);
    complete_url.push(name);

    info!("Pushing image: {}...", name);
    
    // Push does not impl stream so have to deal with less data for pushing containers
    // TODO -- write own function using docker API to push containers
    match docker.images().push(&complete_url.to_string_lossy(), &PushOptions::builder().auth(auth).build()).await {
        Ok(stream) => {
            stream
        },
        Err(e) => {
            error!("Error pushing image");
            error!("Trace: {:?}", e);
            return Err(e.to_string());
        },
    };

    info!("Pushed image: {}", name);
    Ok(())
}

/// Pulls image from remote registry specified by `DOCKER_REGISTRY_URL` env var
/// 
/// Authenticates with the `DOCKER_REGISTRY_USERNAME` and `DOCKER_REGISTRY_PASSWORD` environment variables
/// 
/// ## Returns
/// - `Ok(())` - Image successfully pulled
/// - `Err(String)` - Error occurred while pulling
pub async fn pull_image(docker: &Docker, name: &str) -> Result<(), String>{
    let registry_username = reg_username();
    let registry_password = reg_password();
    let registry_url = reg_password();

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();
        
    let mut complete_url = PathBuf::from(registry_url);
    complete_url.push(name);

    info!("Attempting to pull image: {}", name);

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
                    },
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

/// Deletes a local Docker image
/// 
/// If image is not found, skips deletion and logs a warning
/// 
/// ## Returns
/// - `Ok(())` - Image successfully deleted
/// - `Err(String)` - Error occurred while deleting
pub async fn delete_image(docker: &Docker, name: &str) -> Result<(), String> {
    info!("Deleting image: {}", name);

    let registry_url = reg_url();
    let mut full_challenge_name = PathBuf::from(registry_url);
    full_challenge_name.push(name);

    match docker.images().get(full_challenge_name.to_string_lossy()).inspect().await {
        Ok(_) => {info!("Image '{}' found", full_challenge_name.to_string_lossy())},
        Err(e) => {
            warn!("Image '{}' not found", full_challenge_name.to_string_lossy());
            debug!("Trace: {:?}", e);
            warn!("Skipping deletion of image: {}", name);
            return Err(e.to_string());
        }    
    };

    match docker.images().get(full_challenge_name.to_string_lossy()).delete().await {
        Ok(_) => {
            info!("Successfully deleted image: {}", name);
            Ok(())
        },
        Err(e) => {
            warn!("Error deleting image");
            error!("Trace: {:?}", e);
            return Err(e.to_string());
        }
    }
}

// /// Helper function to just simplify and clean up environment variable fetching
// fn get_env(env_name: &str) -> Result<String, String> {
//     match env::var(env_name) {
//         Ok(val) => Ok(val.to_string()),
//         Err(e) => {
//             error!("Error reading \"{}\" env var", env_name);
//             debug!("Trace: {:?}", e);
//             return Err(e.to_string());
//         }
//     }
// }