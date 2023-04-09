use env::chall_folder_default;
use k8s_openapi::{api::{core::v1::{Pod, Service, Secret}, 
                  apps::v1::Deployment}};
use kube::{Client, Api, Error, 
           core::{ObjectList},
           api::{ListParams, PostParams, DeleteParams}};
use std::{fs::{File}, io::Read, path::PathBuf, collections::{HashMap}};
pub mod network_protocol;
mod env;

use network_protocol::*;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

use crate::env::{reg_username, reg_password, reg_url};
pub use env::check_env_vars;

// BIG TODOS --> 
// GET RID OF CHALL FOLDER PATH REFERENCES, MIGRATE TO ENV VAR - DONE
// MERGE DUPLICATE CODE SECTIONS 
// CLEAN UP CODE IN GENERAL, THIS REALLY SUCKS
// IMPROVE LOGGING
// CHECK OUT LOAD BALANCING (not priority)
// Right now, if it tries creating a deployment but does not have image locally, it doesn't say anything - not a huge deal since pulling will throw problem

// todo --> fix error propagation, make them return not strings and an actual error type 
/// Retrieves challenge parameters for a given challenge, given a name and containing folder
/// 
/// ## Returns
/// - `Ok(HashMap<&'static str, ChallengeParams>)` - HashMap of challenge names to challenge parameters for each service type
/// - `Err(String)` - Error trace if error occurs
fn fetch_challenge_params(name: &str, chall_folder_path: Option<&str>) -> Result<HashMap<&'static str, ChallengeParams>, String> {
    let chall_folder = get_chall_folder(chall_folder_path);
    
    let mut yaml_path = PathBuf::from(chall_folder);
    yaml_path.push(name);
    yaml_path.push("chall.yaml");

    let mut yaml_file = match File::open(&yaml_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Error opening yaml file");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    let mut yaml_string = String::new();
    match yaml_file.read_to_string(&mut yaml_string) {
        Ok(_) => (),
        Err(err) => {
            error!("Error reading yaml file into buffer");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        },
    };

    let deserialized: YamlFile = match serde_yaml::from_str(&yaml_string) {
        Ok(deser) => deser,
        Err(err) => {
            error!("Error deserializing yaml file");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        },
    };
    
    let web = deserialized.deploy.web;
    let admin = deserialized.deploy.admin;
    let nc = deserialized.deploy.nc;

    let deploy_service_types: HashMap<&str, ChallengeParams> = [
        ("web", web),
        ("admin", admin),
        ("nc", nc),
    ]
        .into_iter()
        .map(|(name, data)| data.map(|data| (name, data)))
        .flatten()
        .collect();

    Ok(deploy_service_types)
}

/// Creates the Kubernetes client to be used for all Kubernetes related functions. 
/// 
/// ## Returns
/// - `Ok(Client)` - Kubernetes client
/// - `Err(String)` - Error trace if error occurs
pub async fn create_client() -> Result<Client, String> {
    match Client::try_default().await {
        Ok(client) => {
            info!("Successfully connected to Kubernetes");
            match generate_registry_secret(client.clone()).await {
                Ok(_) => {
                    info!("Successfully created Docker registry secret");
                    Ok(client)
                },
                Err(err) => {
                    error!("Error creating registry secret");
                    warn!("Ensure Kubernetes cluster is running");
                    debug!("Trace: {:?}", err);
                    Err(err.to_string())
                }
            }
        },
        Err(err) => {
            error!("Error creating Kubernetes client");
            debug!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

pub async fn get_pods(client : Client) -> Result<ObjectList<Pod>, String> {
    let pods: Api<Pod> = Api::default_namespaced(client);
    match pods.list(&ListParams::default()).await {
        Ok(pods) => {
            Ok(pods)
        }, 
        Err(err) => {
            error!("Error retrieving pods");
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

// TODO --> Add support for admin bot stuff
// TODO --> Return list of challenges with their respective addresses to access (look into load balancer ingresses and such)

/// Sets up a full Kubernetes deployment for every challenge in `name_list`. 
/// 
/// `chall_folder_path` is the base challenge directory where all challenges are contained in.
/// 
/// ## Parameters
/// - `client` : `Client` 
///     - Kubernetes client
/// - `name_list` : `Vec<&str>`
///     - List of challenge names to deploy
/// - `chall_folder_path` - `Option<&str>`
///     - Path to the base challenge directory, if `None`, defaults to environment variable `CHALL_FOLDER_PATH`
/// 
/// ## Returns
/// - `Ok(Vec<i32>)` - Vector with all corresponding exposed port numbers of each challenge deployed
/// - `Err(String)` - Error trace if error occurs
pub async fn create_challenge(client : Client, name_list : Vec<&str>, chall_folder_path: Option<&str>) -> Result<Vec<i32>, String> {    
    let mut port_list = Vec::new();
    for name in name_list {
        info!("Creating challenge {:?}", name);
        
        let _deployment = match create_deployment(client.clone(), name, chall_folder_path).await {
            Ok(deployment) => {
                deployment
            }
            Err(err) => {
                error!("Error creating deployment");
                info!("Trace: {:?}", err);
                return Err(err.to_string());
            }
        };
    
        let service = match create_service(client.clone(), name, chall_folder_path).await {
            Ok(service) => service,
            Err(err) => {
                error!("Error creating service");
                info!("Trace: {:?}", err);
                return Err(err.to_string());
            }
        };
        // basically all this does is returns the port that the service is listening on externally
        let service_port = match service.spec {
            Some(status) => {
                match status.ports {
                    Some(ports) => {
                        match ports[0].node_port {
                            Some(port) => port,
                            None => {
                                error!("No service node_port found");
                                return Err("Error retrieving service node_port".to_string());
                            }
                        }
                    },
                    None => {
                        error!("Error retrieving service ports");
                        return Err("Error retrieving service ports".to_string());
                    }
                }
            },
            None => {
                error!("No service spec found");
                return Err("No service spec found".to_string());
            }
        };

        // add in tcp/udp differences
        // maybe look into LoadBalancer ingress to get external ip as well
        info!("Challenge {:?} successfully created --> port {}", name, service_port);
        port_list.push(service_port);
    }
    Ok(port_list)
}

// TODO --> Add a check to see if there is more than 1 replica, and if so, set up a loadBalancer for that chall instead of a nodePort

/// Creates a Kubernetes [`Service`][Service] with name `<ChallengeName>-service` for a given challenge
/// 
/// This function serves mostly as a wrapper around [`create_schema_service`][create_schema_service], which generates the schema for the [`Service`][Service].
/// If a service already exists, deletes the existing one and recreates it.
/// 
/// ## Returns
/// - `Ok(Service)` - Kubernetes [`Service`][Service] object
/// - `Err(String)` - Error trace if error occurs
async fn create_service(client: Client, name : &str, chall_folder_path: Option<&str>) -> Result<Service, String> {
    let services: Api<Service> = Api::default_namespaced(client.clone());
    let service_name = format!("{}-service", name);

    let chall_params = match fetch_challenge_params(name, chall_folder_path) {
        Ok(chall_params) => chall_params,
        Err(err) => {
            error!("Error fetching challenge params");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    // TODO --> THIS DOES NOT SUPPORT ADMIN BOTS YET 
    let data_service : Service;
    if let Some(params) = chall_params.get("web") {
        data_service = create_schema_service(name, params).await?;
    } else if let Some(params) = chall_params.get("nc") {
        data_service = create_schema_service(name, params).await?;
    } else if let Some(_params) = chall_params.get("admin") {
        todo!("Admin bots not yet supported");
    } else {
        error!("Error creating service schema, check yaml");
        return Err("Error creating service schema, check yaml".to_string());
    }

    match service_exists(client.clone(), name).await {
        Ok(status) => {
            if status {
                warn!("Service already exists, deleting");
                match delete_service(client.clone(), name).await {
                    Err(err) => {
                        return Err(err.to_string());
                    },
                    _ => ()
                }
            }
        }, 
        Err(err) => {
            return Err(err.to_string());
        } 
    };

    match services.create(&PostParams::default(), &data_service).await {
        Ok(service_instance) => {
            info!("Service {} created", service_name);
            Ok(service_instance)
        }
        Err(err) => {
            error!("Error creating service");
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

/// Generates Kubernetes `Secret` that allows it to authenticate with the remote Docker registry to pull images
/// 
/// Secret name generated is `container-registry-credentials` and is stored in the default namespace
/// 
/// ## Returns
/// - `Ok(Secret)` - Kubernetes [`Secret`][Secret] object
/// - `Err(String)` - Error trace if error occurs
async fn generate_registry_secret(client: Client) -> Result<Secret, String>{
    info!("Generating remote Docker registry secret...");
    let secrets: Api<Secret> = Api::default_namespaced(client.clone());

    match secret_exists(client.clone(), "container-registry-credentials").await {
        Ok(status) => {
            if status {
                warn!("Registry secret already exists, deleting");
                match delete_secret(client.clone(), "container-registry-credentials").await {
                    Err(err) => {
                        return Err(err.to_string());
                    },
                    _ => ()
                }
            }
        },
        Err(err) => {
            error!("Error checking if Docker registry secret exists");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    let registry_username = reg_username();
    let registry_password = reg_password();
    let registry_url = reg_url();
    // let registry_username = get_env("DOCKER_REGISTRY_USERNAME")?;
    // let registry_password = get_env("DOCKER_REGISTRY_PASSWORD")?;
    // let registry_url = get_env("DOCKER_REGISTRY_URL")?;

    let encoded = base64::encode(format!("{}:{}", registry_username, registry_password));

    let dockerconfigdata : String = "{\"auths\":{\"".to_owned() + &registry_url + &"\":{\"username\":\"".to_owned() + &registry_username + "\",\"password\":\"" + &registry_password + "\",\"auth\":\"" + &encoded + "\"}}}";
    let base_encoded_dockerconfigdata : String = base64::encode(dockerconfigdata);

    let secret : Result<Secret, String> = match serde_json::from_value(serde_json::json!({
            "apiVersion": "v1",
            "data": {
                ".dockerconfigjson": base_encoded_dockerconfigdata
            },
            "kind": "Secret",
            "metadata": {
                "name": "container-registry-credentials",
                "namespace": "default"
            },
            "type": "kubernetes.io/dockerconfigjson"
        }
    )) {
        Ok(data_deploy) => {
            Ok(data_deploy)
        },
        Err(err) => {
            error!("Error generating json for Docker registry secret");
            debug!("TRACE: {:?}", err);
            Err(err.to_string())
        }
    };

    match secrets.create(&Default::default(), &secret?).await {
        Ok(secret) => {
            Ok(secret)
        },
        Err(err) => {
            error!("Error creating secret with json data");
            debug!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

// TODO - migrate schema to a separate file for organizational purposes
/// Generates Service object with name `<ChallengeName>-service` from the current service schema for a given challenge
/// 
/// ## Returns 
/// - `Ok(Service)` - Kubernetes [`Service`][Service] object
/// - `Err(String)` - Error trace if error occurs
async fn create_schema_service(name: &str, params: &ChallengeParams) -> Result<Service, String> {
    let service_name = format!("{}-service", name);
    match serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": service_name,
            "labels": {
                "app": service_name
            }
        },
        "spec": {
            "ports": [
                {
                    "port": params.expose.port(),
                    "targetPort": params.expose.port(),
                    "protocol": params.expose.protocol()
                }
            ],
            "selector": {
                "app": name
            },
            "type": "NodePort"
        }
    })) {
        Ok(data_service) => return Ok(data_service),
        Err(err) => {
            error!("Error creating schema for service");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
}

// TODO - migrate schema to a separate file for organizational purposes
/// Generates a Kubernetes [`Deployment`][Deployment] object for a given challenge
/// 
/// If a deployment already exists, it will be deleted and recreated
/// 
/// ## Returns
/// - `Ok(Deployment)` - Kubernetes [`Deployment`][Deployment] object
/// - `Err(String)` - Error trace if error occurs
async fn create_deployment(client: Client, name: &str, chall_folder_path: Option<&str>) -> Result<Deployment, String> {
    let deployments: Api<Deployment> = Api::default_namespaced(client.clone());

    info!("Creating deployment");
    let chall_params = match fetch_challenge_params(name, chall_folder_path) {
        Ok(chall_params) => chall_params,
        Err(err) => {
            error!("Error fetching challenge params");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    let data_deploy: Deployment;
    // goes and checks each subsection for yaml, if web chall, creates schema for web, if admin, creates schema for admin, etc.
    // can add custom ones in the future if need be here
    if let Some(params) = chall_params.get("web") {
        data_deploy = create_schema_deployment(name, params)?;
    } else if let Some(params) = chall_params.get("nc") {
        data_deploy = create_schema_deployment(name, params)?;
    } else if let Some(_params) = chall_params.get("admin") {
        todo!("Admin bots not yet supported");
    } else {
        error!("Error creating service schema, check yaml and ensure either \"web\", \"nc\" or \"admin\" are specified");
        return Err("Error creating service schema, check yaml".to_string());
    }

    match deploy_exists(client.clone(), name).await {
        Ok(status) => {
            if status {
                warn!("Deployment already exists, deleting");
                match delete_deployment(client.clone(), name).await {
                    Err(err) => {
                        error!("Error deleting deployment");
                        debug!("Trace: {:?}", err);
                        return Err(err.to_string());
                    },
                    _ => ()
                }
            }
        }, 
        Err(err) => {
            return Err(err.to_string());
        } 
    };
    // TODO --> make it wait for deployment to be ready?
    match deployments.create(&PostParams::default(), &data_deploy).await {
        Ok(deployment_instance) => {
            info!("Deployment {} created", name);
            Ok(deployment_instance)
        },
        Err(err) => {
            error!("Error creating deployment {}", name);
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

/// Generates a Kubernetes [`Deployment`][Deployment] object from the current deployment schema for a given challenge
/// 
/// ## Returns
/// - `Ok(Deployment)` - Kubernetes [`Deployment`][Deployment] object
/// - `Err(String)` - Error trace if error occurs
fn create_schema_deployment(name: &str, chall_params: &ChallengeParams) -> Result<Deployment, String>{
    let registry_url = reg_url();

    let mut path_on_registry = PathBuf::new();
            path_on_registry.push(registry_url);
            path_on_registry.push(name);

    match serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": name,
            "labels": {
                "app": name
            }
        },
        "spec": {
            "replicas": chall_params.replicas,
            "selector": {
                "matchLabels": {
                    "app": name
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": name
                    }
                },
                "spec": {
                    "containers": [
                            {
                                "name": name,
                                "image": path_on_registry.to_str().unwrap(),
                                "imagePullPolicy": "Always",
                                "ports": [
                                    {
                                        "containerPort": chall_params.expose.port(),
                                        "protocol": chall_params.expose.protocol()
                                    },
                                ]
                            }
                        ],
                    "imagePullSecrets": [
                            {
                                "name": "container-registry-credentials"
                            }
                        ]
                    }
                }
            }
        }
    )) {
        Ok(data_deploy) => {
            return Ok(data_deploy);
        },
        Err(err) => {
            error!("Error creating deployment schema");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
}

// TODO --> Merge delete deployment and service into one function, secret might not be as easy but possible
pub async fn delete_deployment(client : Client, name : &str) -> Result<(), String> {
    info!("Deleting deployment {:?}", name);
    let deployments: Api<Deployment> = Api::default_namespaced(client.clone());
    deployments.delete(name, &DeleteParams::default()).await.unwrap();
    info!("Successfully deleted deployment {:?}", name);
    Ok(())
}

pub async fn delete_service(client: Client, name : &str) -> Result<(), String> {
    info!("Deleting service {:?}", name);
    let services: Api<Service> = Api::default_namespaced(client.clone());
    services.delete(format!("{}-service", name).as_str(), &DeleteParams::default()).await.unwrap();
    info!("Successfully deleted service {:?}", name);
    Ok(())
}

pub async fn delete_secret(client: Client, name : &str) -> Result<String, String> {
    info!("Deleting Kubernetes secret \"{}\"...", name);
    let secrets: Api<Secret> = Api::default_namespaced(client.clone());
    let status = match secrets.delete(name, &DeleteParams::default()).await {
        Ok(delete_status) => {
            delete_status
        },
        Err(err) => {
            error!("Error deleting secret");
            debug!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
        
    match status.right() {
        Some(status) => {
            if status.status == "Success" {
                info!("Successfully deleted secret {:?}", name);
                return Ok("Successfully deleted secret".to_string());
            } else {
                error!("Error deleting secret {:?}", name);
                debug!("{:?}", status);
                return Err("Error deleting secret".to_string());
            }
        },
        None => {
            error!("Error deleting secret {:?}", name);
            return Err("Error deleting secret".to_string());
        }
    }
}

pub async fn delete_challenge(client : Client, name_list : Vec<&str>) -> Result<(), String> {
    for name in name_list {
        info!("Deleting challenge {:?}", name);

        let dep_exists = match deploy_exists(client.clone(), name).await {
            Ok(deploy_exists) => deploy_exists,
            Err(err) => {
                error!("Error checking if deployment exists");
                info!("Trace: {:?}", err);
                return Err(err.to_string());
            }
        };
        
        let serv_exists = match service_exists(client.clone(), name).await {
            Ok(service_exists) => service_exists,
            Err(err) => {
                error!("Error checking if service exists");
                info!("Trace: {:?}", err);
                return Err(err.to_string());
            }
        };
    
        if dep_exists {
            delete_deployment(client.clone(), name).await?;
        } else {
            warn!("Skipping...deployment {:?} does not exist", name);
        }
        
        if serv_exists {
            delete_service(client.clone(), name).await?;
        } else {
            warn!("Skipping...service {:?} does not exist", format!("{}-service", name));
        }
    
        info!("Successfully deleted challenge {:?}", name);
    }
    
    Ok(())
}


// TODO - Reduce down to one function
async fn deploy_exists(client: Client, name : &str) -> Result<bool, Error> {
    let deployments: Api<Deployment> = Api::default_namespaced(client.clone());
    if let Some(_deployment) = deployments.get_opt(name).await? {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

async fn service_exists(client: Client, name : &str) -> Result<bool, Error> {
    let services: Api<Service> = Api::default_namespaced(client.clone());
    if let Some(_service) = services.get_opt(format!("{}-service", name).as_str()).await? {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

async fn secret_exists(client: Client, name : &str) -> Result<bool, Error> {
    let secrets: Api<Secret> = Api::default_namespaced(client.clone());
    if let Some(_secret) = secrets.get_opt(name).await? {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

// TODO --> Improve environment var system
/// Helper function to simplify environment var fetching
/// 
/// ## Returns
/// - `Ok(String)` - Contents of the given environment variable with `env_name`
/// - `Err(String)` - Error trace that occurred when trying to read the variable
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

/// Helper function to simplify fetching the base challenge folder
/// 
/// If no `chall_folder_path` specified, defaults the path to the `CHALL_FOLDER` environment variable
/// 
/// ## Returns
/// - `Ok(String)` - Path to the challenge folder
/// - `Err(String)` - Error trace that occurred when trying to get a folder path
pub fn get_chall_folder(chall_folder_path: Option<&str>) -> String {
    if let Some(path) = chall_folder_path {
        path
    } else {
        debug!("Using default chall folder");
        chall_folder_default()
    }.to_string()
    // match chall_folder_path {
    //     Some(chall_folder_path) => Ok(chall_folder_path.to_string()),
    //     None => chall_folder_default() {
    //             Ok(path) => Ok(path.to_string()),
    //             Err(e) => {
    //                 error!("Error getting challenge folder path");
    //                 info!("Trace: {:?}", e);
    //                 Err(e.to_string())
    //             }
    //         }
    //     }
    // }
}