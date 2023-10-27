use arcs_yaml_parser::YamlShape;
use arcs_yaml_parser::files::structs::ContainerType;

use crate::server::responses::Metadata;
use crate::env::s3_display_address;
use crate::logging::*;

pub fn take_only_rightmost_segment(text: &str) -> &str {
    text.rsplit_once('/').map(|(_, right)| right).unwrap_or(text)
}

pub fn get_static_file_links(meta: &Metadata, yaml: &YamlShape) -> Result<Vec<String>, String> {
    let mut static_file_links : Vec<String> = Vec::new();

    let files = yaml
        .file_iter()
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();

    let chall = meta.chall_name().trim_matches('/');
    let base = s3_display_address().trim_matches('/');

    // TODO --> improve error messages on these branches 
    for file in files {
        info!("FILE: {:?}", file);

        let Some(container_type) = file.container() else {
            trace!("Adding regular static file");
            let Some(file_path) = file.path().to_str() else {
                return Err("Failed to parse file path".to_string());
            };
            
            let file_name = take_only_rightmost_segment(file_path);
            debug!("file_name: {file_name}");

            static_file_links.push(format!("{base}/{chall}/{file_name}"));
            continue;
        };

        match container_type {
            ContainerType::Nc => {
                let Some(file_path) = file.path().to_str() else {
                    return Err("Failed to find file path for file".to_string());
                };
                let Some((_, filename)) = file_path.rsplit_once('/') else {
                    return Err("Failed to parse file name for file path".to_string());
                };
                static_file_links.push(format!("{base}/{chall}/{filename}"));
            }
            // If in the future there are other weird container files, add more branches here
            _ => {
                let Some(file_path) = file.path().to_str() else {
                    return Err("Failed to find file path for file".to_string());
                };
                let Some((_, name)) = file_path.rsplit_once('/') else {
                    return Err("Failed to parse file name for file path".to_string());
                };
                static_file_links.push(format!("{base}/{chall}/{name}"));
            }
        }
    }

    info!("STATIC FILE LINKS: {:?}", static_file_links);
    Ok(static_file_links)
}
