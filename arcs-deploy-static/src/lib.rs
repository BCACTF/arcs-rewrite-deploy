use std::io::Read;
use std::{path::PathBuf, fs::read_to_string};

use arcs_deploy_docker::fetch_container_file;
use reqwest::Client;

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub mod env;
use env::*;

use arcs_yaml_parser::{YamlShape, YamlVerifyError, File};
use arcs_yaml_parser::files::structs::ContainerType;
use shiplift::Docker; 

// TODO --> Move this into yaml crate
pub fn fetch_chall_yaml(chall_folder_name: &str) -> Option<Result<YamlShape, YamlVerifyError>> {
    let folder_path = PathBuf::from_iter([chall_folder_default(), chall_folder_name]);
    let yaml_path = folder_path.join("chall.yaml");
    let yaml_data = read_to_string(&yaml_path).ok()?;

    Some(YamlShape::try_from_str(&yaml_data, &Default::default(), Some(&folder_path)))
}

pub async fn deploy_static_files(docker: &Docker, chall_name: &str) -> Result<Vec<File>, Vec<File>> {
    info!("Deploying static challenge: {}", chall_name);
    let client = Client::new();

    let yaml = match fetch_chall_yaml(chall_name) {
        Some(yaml_result) => {
                match yaml_result {
                    Ok(yaml) => yaml,
                    Err(e) => {
                        error!("Failed to parse chall.yaml for {}: {:#?}", chall_name, e);
                        return Err(vec![]);
                    },
                }
            },
        None => {
            error!("Failed to find chall.yaml for challenge: {}", chall_name);
            return Err(vec![]);
        }
    };

    let files: Vec<File> = yaml
        .file_iter()
        .into_iter()
        .flatten()
        .cloned()
        .collect();


    let mut success = vec![];
    let mut failure = vec![];

    for file in files {
        let url = {
            let base = s3_bucket_url().trim_matches('/');
            let chall = chall_name.trim_matches('/');
    
            let file = if let Some(file_path) = file.path().to_str() {
                if let Some((_, name)) = file_path.rsplit_once("/") {
                    name
                } else {
                    file_path
                }
            } else {
                failure.push(file);
                continue;
            };

            info!("{:?}", file);

            format!("{base}{chall}/{file}")
        };

        let filedata: Vec<u8> = if let Some(containertype) = file.container() {
            match containertype {
                // kinda a big hack but it works for now
                ContainerType::Nc => {
                    info!("Deploying files in container for challenge: {}", chall_name);
                    match fetch_container_file(docker, chall_name, file.path()).await {
                        Ok(returned_filedata) => {
                            let mut archive = tar::Archive::new(returned_filedata.as_slice());
                            let mut entries = match archive.entries() {
                                Ok(entries) => entries,
                                Err(e) => {
                                    error!("Failed to fetch tar entires from container file: {:#?}", e);
                                    return Err(vec![]);
                                },
                            };

                            let specific_entry = entries.find(
                                |entry| {
                                    info!("Checking an entry...");
                                    if let Ok(entry) = entry {
                                        if let Ok(entry_path) = entry.path() {
                                            info!("Entry path: {:?}", entry_path);
                                            if let Some(entry_path) = entry_path.to_str() {
                                                if let Some(file_path) = file.path().to_str() {
                                                    if let Some((_, filename)) = file_path.rsplit_once("/") {
                                                        info!("Filename: {:?}", filename);
                                                        return entry_path.contains(filename);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    false
                                }
                            );

                            let file_bytes = match specific_entry {
                                Some(entry) => {
                                    match entry {
                                        Ok(mut entry) => {
                                            let mut filedata = vec![];
                                            if let Err(e) = entry.read_to_end(&mut filedata) {
                                                error!("Failed to read file from container: {:#?}", e);
                                                return Err(vec![]);
                                            }
                                            filedata
                                        },
                                        Err(e) => {
                                            error!("Failed to fetch file from container: {:#?}", e);
                                            return Err(vec![]);
                                        },
                                    }
                                },
                                None => {
                                    error!("Error getting entry from list of entries");
                                    return Err(vec![]);
                                }
                            };

                            file_bytes
                        }, 
                        Err(e) => {
                            error!("Failed to fetch file from container: {:#?}", e);
                            return Err(vec![]);
                        },
                    }
                },
                // TODO --> could add specific messages depending on container type specified here
                _ => {
                    file.data_vec_cloned()
                }
            }
        } else {
            file.data_vec_cloned()
        };

        let res = client
            .post(&url)
            .bearer_auth(s3_bearer_token())
            .body(filedata)
            .send()
            .await;

        match res {
            Ok(res) if res.status().is_success() => success.push(file),
            error => {
                error!("Failed to upload file: {:#?}", error);
                warn!("Ensure CDN auth token is valid.");
                failure.push(file)
            }
        }
    }

    if failure.is_empty() {
        Ok(success)
    } else {
        Err(failure)
    }
}