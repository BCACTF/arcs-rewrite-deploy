use env::chall_folder_default;
use futures::TryStreamExt;
use shiplift::container::ContainerInfo;
use shiplift::image::ImageBuildChunk;
use std::path::Path;
use std::fs::read_dir;

use shiplift::{Docker, image::{PushOptions, PullOptions, BuildOptions, ImageInfo}};

use std::default::Default;
use std::path::PathBuf;

mod env;
pub use env::check_env_vars;

use futures::stream::StreamExt;
#[allow(unused_macros)]
pub mod logging {
    use arcs_logging_rs::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

use crate::env::{reg_url, reg_username, reg_password};

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



// TODO --> fix error propagation, make them return not strings and an actual error type 
// todo --> update documentation for this function
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
pub async fn build_image(docker: &Docker, chall_folder_name : &str, inner_path: Option<&Path>) -> Result<(), String> {
    let challenge_folder = chall_folder_default();
    let registry_url = reg_url();

    info!("Creating image for: {:?}", chall_folder_name);

    let challenge_path: PathBuf;
    let full_registry_path: PathBuf;

    if let Some(sub_chall_folder) = inner_path {
        challenge_path = PathBuf::from_iter([Path::new(challenge_folder), Path::new(chall_folder_name), sub_chall_folder]);
        full_registry_path = PathBuf::from_iter([Path::new(registry_url), Path::new(chall_folder_name), sub_chall_folder]);
    } else {
        challenge_path = PathBuf::from_iter([Path::new(challenge_folder), Path::new(chall_folder_name)]);
        full_registry_path = PathBuf::from_iter([Path::new(registry_url), Path::new(chall_folder_name)]);
    }

    let build_options = BuildOptions::builder(challenge_path.to_string_lossy().to_string())
        .tag(full_registry_path.to_string_lossy())
        .dockerfile("Dockerfile")
        .rm(true)
        .build();
        
    let mut stream = docker.images().build(&build_options);
    while let Some(build_result) = stream.next().await {
        match build_result {
            Ok(output) => {
                match &output {
                    ImageBuildChunk::Update {stream} => {
                        trace!("{:?}", stream);
                    },
                    ImageBuildChunk::Error {error, ..} => {
                        error!("Error building {:?}", chall_folder_name); // if this is a subfolder error, just says challname
                        debug!("Trace: {:?}", error);
                        warn!("Skipping challenge {:?}, check logs for details...", chall_folder_name);
                        return Err(error.to_string());
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
                return Err(e.to_string());
            },
        }
    }
    
    info!("{:?} image has been built", chall_folder_name); // if this is a subfolder error, just says challname

    Ok(())
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
            Err(err.to_string())
        }
    }
}

// TODO --> Improve using yaml, iterate through all dirs, if Deploy field specified, follow path and deploy
/// Quality of life function that builds all images with Dockerfiles that it finds in the `CHALL_FOLDER` direectory
pub async fn build_all_images(docker : &Docker) -> Result<String, String> {
    match fetch_chall_folder_names() {
        Ok(names) => {
            let available_challs_to_deploy : Vec<&str> = names.iter().map(AsRef::as_ref).collect();
            info!("Attempting to build all challenges...");
            for chall in &available_challs_to_deploy {
                info!("Building {:?}", chall);
                build_image(docker, chall, None).await?;
            }
            info!("Successfully built all images.");
            Ok("Successfully built all images.".to_string())
        },
        Err(err) => {
            error!("Error fetching folder names");
            debug!("Trace: {}", err);
            Err(err)
        }
    }
}

// TODO --> Write own push function that impl stream to better update users on progress
/// Pushes image to remote registry specified by `DOCKER_REGISTRY_URL` env var
/// 
/// Authenticates with the `DOCKER_REGISTRY_USERNAME` and `DOCKER_REGISTRY_PASSWORD` environment variables
/// 
/// ## Returns
/// - `Ok(())` - Image successfully pushed
/// - `Err(String)` - Error occurred while pushing
pub async fn push_image(docker: &Docker, name: &str, inner_path: Option<&Path>) -> Result<(), String> {
    let registry_username = reg_username();
    let registry_password = reg_password();
    let registry_url = reg_url();

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();

    let mut complete_url = PathBuf::from(registry_url);
    
    complete_url.push(name);
    
    if let Some(path) = inner_path {
        complete_url.push(path);
        info!("Pushing image with inner_path: {}/{}", name, path.to_string_lossy());
    } else {
        info!("Pushing image: {}...", name);
    }
    
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
pub async fn pull_image(docker: &Docker, name: &str, inner_path: Option<&Path>) -> Result<(), String>{
    let registry_username = reg_username();
    let registry_password = reg_password();
    let registry_url = reg_url();

    let auth = shiplift::RegistryAuth::builder()
        .username(registry_username)
        .password(registry_password)
        .server_address(registry_url)
        .build();
        
    let mut complete_url = PathBuf::from(registry_url);
    complete_url.push(name);

    if let Some(path) = inner_path {
        complete_url.push(path);
        info!("Attempting to pull image with inner_path: {}/{}", name, path.to_string_lossy());
    } else {
        info!("Attempting to pull image: {}", name);
    }

    let mut stream = docker.images().pull(&PullOptions::builder().auth(auth).image(complete_url.to_string_lossy()).build());
    while let Some(data) = stream.next().await {
        match data {
            Ok(output) => {
                match &output {
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
pub async fn delete_image(docker: &Docker, name: &str, inner_path: Option<&Path>) -> Result<(), String> {
    info!("Deleting image: {}", name);

    let registry_url = reg_url();
    let mut full_challenge_name = PathBuf::from(registry_url);
    full_challenge_name.push(name);

    if let Some(path) = inner_path {
        full_challenge_name.push(path);
    }
    
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
            Err(e.to_string())
        }
    }
}
// TODO --> make this nicer, feels really hacky atm
pub async fn fetch_container_file(docker: &Docker, container_name: &str, file_path: &Path) -> Result<Vec<u8>, String> {
    let reg_url = reg_url();
    
    warn!("IMAGE REQUESTING FILE INSIDE CONTAINER RUNNING...");
    let image_name = format!("{}/{}", reg_url, container_name);

    use shiplift::container::ContainerOptions;
    let opts = ContainerOptions::builder(&image_name).build();

    let containers = docker.containers();

    if let Ok(create_result) = containers.create(&opts).await {
        info!("Successfully started docker container for image {}", image_name);
        containers.get(create_result.id);
    } else {
        warn!("Could not create the docker container in the current system. Image name: {}", image_name);
        return Err(format!("Could not create the docker container in the current system."));
    };

    let container_info = if let Some(container) = retrieve_containers(docker).await?
        .into_iter()
        .find(|container| {
            let non_id_name = if let Some((name, _)) = container.image.split_once("@") {
                name
            } else {
                container.image.as_str()
            };
            non_id_name == format!("{}/{}", reg_url, container_name)
        })
        {
            container
        } else {
            error!("Container '{}' not found", container_name);
            return Err(format!("Container '{}' not found", container_name));
        };

    let container = docker.containers().get(container_info.id);

    let file = container.copy_from(file_path);
    let data = match file.try_concat().await {
        Ok(data) => data,
        Err(e) => {
            error!("Error fetching file within Docker container");
            debug!("Trace: {:?}", e);
            return Err(e.to_string());
        }
    };
    
    Ok(data)
}
