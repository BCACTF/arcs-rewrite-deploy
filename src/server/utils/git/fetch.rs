use crate::{logging::*, env::git_branch};
use super::{
    GitResult,
    signature_auth::get_auth_callbacks,
};

use git2::Remote;

pub fn fetch_from_remote(remote: &mut Remote) -> GitResult {
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(get_auth_callbacks());

    let duration = std::time::UNIX_EPOCH.elapsed().ok();

    let message = if let Some(duration) = duration {
        let hours = duration.as_secs() / (60*60) % 24;
        let minutes = duration.as_secs() / 60 % 60;
        let seconds = duration.as_secs() % 60;

        format!("Fetching new remote commits at {hours:02}:{minutes:02}:{seconds:02} UTC")
    } else {
        "Fetching new remote commits (time unknown)".to_string()
    };

    remote.fetch(&[git_branch()], Some(&mut fo), Some(&message))?;
    trace!("Downloaded commits/changes from remote");

    remote.disconnect()?;
    trace!("Disconnected from remote");

    Ok(())
}
