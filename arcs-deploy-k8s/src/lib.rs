use k8s_openapi::{api::{core::v1::{Pod, Service}, apps::v1::Deployment}, serde_json::json};
use kube::{Client, Api, 
           core::{ObjectList, ObjectMeta},
           api::{ListParams, PostParams, DeleteParams}, Error};
use kube_runtime::wait::{await_condition, conditions};
use std::{fs::{self, File}, io::Read};


#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

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

pub async fn create_challenge(client : Client, name : &str) -> Result<(), String> {
    info!("Creating challenge {:?}", name);
    
    let deployment = match create_deployment(client.clone(), name).await {
        Ok(deployment) => {
            deployment
        }
        Err(err) => {
            error!("Error creating deployment");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    let service = match create_service(client.clone(), name).await {
        Ok(service) => service,
        Err(err) => {
            error!("Error creating service");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

    info!("Challenge {:?} successfully created", name);
    Ok(())
}

async fn create_service(client: Client, name : &str) -> Result<Service, String> {
    let services: Api<Service> = Api::default_namespaced(client.clone());
    let service_name = format!("{}-service", name);
    let data_service = match serde_json::from_value(serde_json::json!({
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
                    "port": 80,
                    "targetPort": 80,
                    "protocol": "TCP"
                }
            ],
            "selector": {
                "app": name
            },
            "type": "NodePort"
        }
    })) {
        Ok(data_service) => data_service,
        Err(err) => {
            error!("Error creating schema for service");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

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

async fn create_deployment(client: Client, name: &str) -> Result<Deployment, String> {
    let deployments: Api<Deployment> = Api::default_namespaced(client.clone());

    info!("Creating deployment");
    let data_deploy = match serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": name,
            "labels": {
                "app": name
            }
        },
        "spec": {
            "replicas": 1,
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
                                        "containerPort": 80,
                                        "protocol": "TCP"
                                    },
                                ]
                            }
                        ]
                    }
                }
            }
        }
    )) {
        Ok(data_deploy) => data_deploy,
        Err(err) => {
            error!("Error creating deployment schema");
            info!("Trace: {:?}", err);
            return Err(err.to_string());
        }
    };

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
            error!("Error creating deployment");
            info!("Trace: {:?}", err);
            Err(err.to_string())
        }
    }
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

pub async fn delete_challenge(client : Client, name : &str) -> Result<(), String> {
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