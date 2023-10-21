use std::io::Read;
use std::{path::PathBuf, fs::read_to_string};

use arcs_deploy_docker::fetch_container_file;

#[allow(unused_macros)]
pub mod logging {
    use arcs_logging_rs::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub mod env;
use env::*;

use arcs_yaml_parser::{YamlShape, YamlVerifyError, File};
use reqwest::header::{HeaderName, HeaderMap};
use s3::Bucket;
use s3::creds::Credentials;
use shiplift::Docker; 

// TODO --> Find a more elegant way to do this
pub fn chall_yaml_path(chall_folder_name: &str) -> PathBuf {
    let folder_path = PathBuf::from_iter([chall_folder_default(), chall_folder_name]);
    folder_path.join("chall.yaml")
}

pub fn fetch_chall_yaml(chall_folder_name: &str) -> Option<Result<YamlShape, YamlVerifyError>> {
    let folder_path = chall_yaml_path(chall_folder_name);
    let yaml_data = read_to_string(&folder_path).ok()?;

    Some(YamlShape::try_from_str(&yaml_data, &Default::default(), Some(&folder_path)))
}

pub async fn get_container_file_data(name: &str, file: &File, docker: &Docker) -> Option<Vec<u8>> {
    if file.container().is_none() {
        return Some(file.data_vec_cloned().unwrap_or_default())
    };
            
    info!("Deploying files in container for challenge: {}", name);
    warn!("At the moment, this is currently NOT functioning if running a multi-server cluster.");
    warn!("To fix, build the container on the same server as this one and redeploy.");

    let container_file = fetch_container_file(docker, name, file.path()).await;
    let Ok(file_data) = container_file else {
        error!("Failed to fetch file from container: {:#?}", container_file);
        return None;
    };

    let mut archive: tar::Archive<&[u8]> = tar::Archive::new(file_data.as_slice());
    let entries = archive.entries();
    let Ok(mut entries) = entries else {
        error!("Failed to fetch tar entires from container file: {:#?}", entries.err());
        return None;
    };

    let specific_entry = entries.find(|entry| {
        info!("Checking an entry...");
        let Ok(entry) = entry else { return false; };
        
        let Ok(entry_path) = entry.path() else { return false; };
        info!("Entry path: {:?}", entry_path);

        let Some(entry_path) = entry_path.to_str() else { return false; };
        
        let Some(file_path) = file.path().to_str() else { return false; };
        let Some((_, filename)) = file_path.rsplit_once("/") else { return false; };
        
        info!("Filename: {:?}", filename);
        entry_path.contains(filename)
    });

    match specific_entry {
        Some(Ok(mut entry)) => {
            let mut filedata = vec![];
            if let Err(e) = entry.read_to_end(&mut filedata) {
                error!("Failed to read file from container: {:#?}", e);
                None
            } else {
                Some(filedata)
            }
            
        },
        Some(Err(e)) => {
            error!("Failed to fetch file from container: {:#?}", e);
            None
        },
        None => {
            error!("Error getting entry from list of entries");
            None
        }
    }
}

pub fn create_s3_client() -> Result<s3::Bucket, s3::error::S3Error> {
    Bucket::new(
        s3_bucket_name(),
        s3::Region::Custom { region: s3_region().to_string(), endpoint: s3_bucket_url().to_string() },
        // Credentials are collected from environment, config, profile or instance metadata
        Credentials::new(Some(s3_access_key()), Some(s3_bearer_token()), None, None, None).unwrap(),
    )
}

// TODO --> if it is not relative (if its a url), add new function flow
pub async fn deploy_static_files(docker: &Docker, chall_name: &str) -> Result<Vec<File>,  Vec<File>> {
    info!("Deploying static challenge: {}", chall_name);

    let bucket = match create_s3_client() {
        Ok(bucket) => bucket,
        Err(e) => {
            error!("Failed to create S3 client: {:#?}", e);
            return Err(vec![]);
        }
    };

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
            trace!("Pushing path: /{}/{}", chall, file);
            format!("/{chall}/{file}")
        };

        let Some(filedata): Option<Vec<u8>> = get_container_file_data(chall_name, &file, docker).await else {
            return Err(vec![]);
        };

        let mut custom_headers = HeaderMap::new();
        custom_headers.insert(
            HeaderName::from_static("x-amz-acl"),
            "public-read".parse().unwrap(),
        );

        let res = bucket.with_extra_headers(custom_headers).put_object(url, &filedata).await;
        

        match res {
            Ok(res) if (res.status_code() == 200) => success.push(file),
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
