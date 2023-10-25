use crate::logging::*;

use super::signature_auth::get_auth_callbacks;

use git2::{
    Remote,
    Repository,

    Error, ErrorCode::NotFound, ErrorClass::Config,
};

use super::GitResult;

pub fn get_remote(repo: &Repository) -> GitResult<Remote> {
    let remotes = repo.remotes()?;
    trace!("Got list of repository remotes");

    let remote_str_bytes = remotes
        .iter_bytes()
        .next()
        .ok_or(Error::new(NotFound, Config, "Unable to find a suitable remote"))?;
    trace!("Got first remote name");
    
    let remote_str = String::from_utf8_lossy(remote_str_bytes);
    debug!("First remote: {remote_str}");
    
    let remote = repo.find_remote(&remote_str)?;
    debug!("Got remote @ {}", String::from_utf8_lossy(remote.url_bytes()));
    
    Ok(remote)
}

pub fn try_get_connected_remote(repo: &Repository) -> GitResult<Option<Remote>> {

    // Fetch repo
    let mut remote = get_remote(repo)?;
    trace!("Got remote");

    let connect_succeeded = remote.connect_auth(
        git2::Direction::Fetch,
        Some(get_auth_callbacks()),
        None,
    ).is_ok();

    if connect_succeeded {
        trace!("Connected to remote");
        Ok(Some(remote))
    } else {
        trace!("Failed to connect to remote");
        Ok(None)
    }
    
}

