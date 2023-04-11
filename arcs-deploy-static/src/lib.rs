use std::{path::PathBuf, fs::read_to_string};

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

// TODO --> Move this into yaml crate
pub fn fetch_chall_yaml(chall_folder_name: &str) -> Option<Result<YamlShape, YamlVerifyError>> {
    let folder_path = PathBuf::from_iter([chall_folder_default(), chall_folder_name]);
    let yaml_path = folder_path.join("chall.yaml");
    let yaml_data = read_to_string(&yaml_path).ok()?;

    Some(YamlShape::try_from_str(&yaml_data, &Default::default(), Some(&folder_path)))
}

pub async fn deploy_static_files(chall_name: &str) -> Result<Vec<File>, Vec<File>> {
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

        let res = client
            .post(&url)
            .bearer_auth(s3_bearer_token())
            .body(file.data_vec_cloned())
            .send()
            .await;

        match res {
            Ok(res) if res.status().is_success() => success.push(file),
            error => {
                error!("Failed to upload file: {:#?}", error);
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