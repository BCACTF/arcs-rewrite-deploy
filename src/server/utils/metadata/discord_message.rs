use yaml::deploy::structs::{DeployLink, DeployTargetType};

use crate::server::responses::Metadata;

use std::fmt::{ Write, Error };

pub fn build_discord_message(
    meta: &Metadata,
    ports: &Option<Vec<(DeployTargetType, Vec<i32>)>>,
    complete_links: &[DeployLink]
) -> Result<String, Error> {
    let mut disc_message = String::with_capacity(240);

    if let Some(ports) = ports {
        writeln!(disc_message, "Successfully deployed **{}** on port(s) {ports:?}", meta.chall_name())?;
    } else {
        writeln!(disc_message, "Successfully deployed **{}**. No ports provided", meta.chall_name())?;
    }

    if !complete_links.is_empty() {
        // TODO --> Maybe make this a bit nicer, isn't really the best way of doing this *probably*
        // Also, for netcat servers, the server this sends out is in the form of an http link which is... not correct.
        for link_to_file in complete_links {
            let link_to_file_link = &link_to_file.link;

            let server_type = link_to_file.deploy_target.resource_type();
            writeln!(disc_message, "{server_type} at: {link_to_file_link}")?;
        }
    }

    Ok(disc_message)
}
