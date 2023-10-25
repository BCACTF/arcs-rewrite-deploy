use crate::logging::*;

use super::GitResult;

use crate::env::git_email;
use crate::env::git_key_path;

use git2::{
    Signature,
    RemoteCallbacks,
    Cred, CredentialType,
    Error,
};

pub fn get_signature() -> GitResult<Signature<'static>> {
    Signature::now("ARCS Admin Panel", git_email())
}

pub fn get_auth_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, username_from_url, cred_type| {
        let username = username_from_url.unwrap_or("git");
        if !cred_type.contains(CredentialType::SSH_KEY) {
            error!("SSH KEYS ARE NOT SUPPORTED (supported: {:?})", cred_type.iter_names().collect::<Vec<_>>());
            return Err(Error::from_str("Repository remote doesn't support SSH keys"));
        }
        let cred_res = Cred::ssh_key(
            username,
            None,
            std::path::Path::new(git_key_path()),
            None,
        );
        cred_res
    });
    callbacks
}
