use std::path::PathBuf;

use arcs_docker::fetch_container_file;

#[allow(unused_macros)]
pub mod logging {
    use arcs_logging_rs::with_target;
    with_target! { "arcs-deploy" }
}

use logging::*;

pub mod env;
use env::*;

use yaml::{YamlShape, YamlVerifyError, File};
use reqwest::header::{HeaderName, HeaderMap};
use s3::Bucket;
use s3::creds::Credentials;
use shiplift::Docker; 

// TODO --> Find a more elegant way to do this
pub fn chall_yaml_path(chall_folder_name: &str) -> PathBuf {
    let folder_path = PathBuf::from_iter([chall_folder_default(), chall_folder_name]);
    folder_path.join("chall.yaml")
}

pub async fn fetch_chall_yaml(chall_folder_name: &str) -> Option<Result<YamlShape, YamlVerifyError>> {
    use tokio::fs::read_to_string;

    let yaml_path = chall_yaml_path(chall_folder_name);
    let folder_path = yaml_path.parent()?;
    let yaml_data = read_to_string(&yaml_path).await.ok()?;

    Some(YamlShape::try_from_str(&yaml_data, &Default::default(), Some(&folder_path)))
}

pub async fn get_container_file_data(name: &str, file: &File, docker: &Docker) -> Option<Vec<u8>> {
    if file.container().is_none() {
        return Some(file.data_vec_cloned().unwrap_or_default())
    };
            
    info!("Deploying files in container for challenge: {}", name);
    warn!("At the moment, this is currently NOT functioning if running a multi-server cluster.");
    warn!("To fix, build the container on the same server as this one and redeploy.");

    let file_fetch_result = fetch_container_file(docker, name, file.path()).await;
    match file_fetch_result {
        Ok(file_data) => Some(file_data),
        Err(e) => {
            error!("Failed to fetch file from container {name}: {:#?}", e);
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

    let yaml = match fetch_chall_yaml(chall_name).await {
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
        let sub_chall_file = if let Some(file_path) = file.path().to_str() {
            if let Some((_, name)) = file_path.rsplit_once("/") {
                name
            } else {
                file_path
            }
        } else {
            failure.push(file);
            continue;
        };

        let s3_path = format!("/{}/{}", chall_name.trim_matches('/'), sub_chall_file);

        let Some(filedata): Option<Vec<u8>> = get_container_file_data(chall_name, &file, docker).await else {
            return Err(vec![]);
        };

        let mut custom_headers = HeaderMap::new();
        custom_headers.insert(
            HeaderName::from_static("x-amz-acl"),
            "public-read".parse().unwrap(),
        );

        let res = bucket.with_extra_headers(custom_headers).put_object(s3_path, &filedata).await;
        

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
