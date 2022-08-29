use k8s_openapi::{api::{core::v1::{Pod, Service}, apps::v1::Deployment}};
use kube::{Client, Api, 
           core::{ObjectList},
           api::{ListParams, PostParams, DeleteParams}, Error};
// use kube_runtime::wait::{await_condition, conditions};
use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::{fs::{ File, read_to_string}, io::Read, path::PathBuf, collections::{BTreeMap, HashMap}};
// use arcs_deploy_docker::{fetch_chall_folder_names};
use dotenv::dotenv;

pub mod network_protocol;
use network_protocol::*;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub fn fetch_challenge_params(name: &str, chall_folder_path: &str) -> Result<HashMap<&'static str, ChallengeParams>, String> {
    let mut yaml_path = PathBuf::new();
    yaml_path.push(chall_folder_path);
    yaml_path.push(name);
    yaml_path.push("chall.yaml");

    let mut yaml_file = match File::open(&yaml_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Error opening yaml file");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    let mut yaml_string = String::new();
    match yaml_file.read_to_string(&mut yaml_string) {
        Err(err) => {
            error!("Error reading yaml file into buffer");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        },
        Ok(_) => ()
    };

    let deser: YamlFile = match serde_yaml::from_str(&yaml_string) {
        Ok(deser) => deser,
        Err(err) => {
            error!("Error deserializing yaml file");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
    
    let web = deser.deploy.web;
    let admin = deser.deploy.admin;
    let nc = deser.deploy.nc;

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

pub async fn create_client() -> Client {
    let client = match Client::try_default().await {
        Ok(client) => {
            info!("Successfully connected to k8s");
            client
        },
        Err(err) => {
            error!("Error creating k8s client");
            info!("Trace: {}", err);
            todo!("handle this");
        }
    };
    client
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

/// TODO --> Add support for admin bot stuff
pub async fn create_challenge(client : Client, name_list : Vec<&str>, chall_folder_path: &str) -> Result<(), String> {
    for name in name_list {
        info!("Creating challenge {:?}", name);
    
        let deployment = match create_deployment(client.clone(), name, chall_folder_path).await {
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
    
        info!("Challenge {:?} successfully created", name);
    }
    Ok(())
}

/// TODO --> Add a check to see if there is more than 1 replica, and if so, set up a loadBalancer for that chall instead of a nodePort
async fn create_service(client: Client, name : &str, chall_folder_path: &str) -> Result<Service, String> {
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
    } else if let Some(params) = chall_params.get("admin") {
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
            info!("Service {}-service created", name);
            Ok(service_instance)
        }
        Err(err) => {
            error!("Error creating service");
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

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
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
}

async fn create_deployment(client: Client, name: &str, chall_folder_path: &str) -> Result<Deployment, String> {
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
    if let Some(params) = chall_params.get("web") {
        data_deploy = create_schema_deployment(name, params)?;
    } else if let Some(params) = chall_params.get("nc") {
        data_deploy = create_schema_deployment(name, params)?;
    } else if let Some(params) = chall_params.get("admin") {
        todo!("Admin bots not yet supported");
    } else {
        error!("Error creating service schema, check yaml");
        return Err("Error creating service schema, check yaml".to_string());
    }


    match deploy_exists(client.clone(), name).await {
        Ok(status) => {
            if status {
                warn!("Deployment already exists, deleting");
                match delete_deployment(client.clone(), name).await {
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

    match deployments.create(&PostParams::default(), &data_deploy).await {
        Ok(deployment_instance) => {
            info!("Deployment {} created", name);
            Ok(deployment_instance)
        }
        Err(err) => {
            error!("Error creating deployment {}", name);
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
}

fn create_schema_deployment(name: &str, chall_params: &ChallengeParams) -> Result<Deployment, String>{
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
                                "image": name,
                                "imagePullPolicy": "Never",
                                "ports": [
                                    {
                                        "containerPort": chall_params.expose.port(),
                                        "protocol": chall_params.expose.protocol()
                                    },
                                ]
                            }
                        ]
                    }
                }
            }
        }
    )) {
        Ok(data_deploy) => {
            info!("{:#?}", data_deploy);
            return Ok(data_deploy);
        },
        Err(err) => {
            error!("Error creating deployment schema");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };
}

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

async fn deploy_exists(client: Client, name : &str) -> Result<bool, Error> {
    let deployments: Api<Deployment> = Api::default_namespaced(client.clone());
    if let Some(deployment) = deployments.get_opt(name).await? {
        return Ok(true);
    } else {
        return Ok(false);
    }
}

async fn service_exists(client: Client, name : &str) -> Result<bool, Error> {
    let services: Api<Service> = Api::default_namespaced(client.clone());
    if let Some(service) = services.get_opt(format!("{}-service", name).as_str()).await? {
        return Ok(true);
    } else {
        return Ok(false);
    }
}