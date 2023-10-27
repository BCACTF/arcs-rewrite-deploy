use arcs_yaml_parser::deploy::structs::{DeployLink, DeployTargetType};

use crate::env::{deploy_address, display_address};

fn address() -> &'static str {
    if let Some(url) = display_address() { return url; }
    if let Some((_, url)) = deploy_address().split_once("://") { return url; }
    deploy_address()
}

pub fn links_from_port_listing(port_descriptors: &Option<Vec<(DeployTargetType, Vec<i32>)>>) -> Vec<DeployLink> {
    let mut links = vec![];

    for (target_type, ports) in port_descriptors.iter().flatten() {
        for port in ports.iter() {
            links.push(
                DeployLink {
                    deploy_target: *target_type,
                    link: if *target_type == DeployTargetType::Nc {
                        format!("{} {}", address(), port)
                    } else {
                        format!("{}:{}", address(), port)
                    },
                }
            );
        }
    }

    links
}