
use std::path::Path;
use git2::build::CheckoutBuilder;
use git2::{Repository, Remote, ObjectType, ResetType, FetchOptions, AutotagOption, Signature, PushOptions};

use crate::server::responses::{Metadata, Response};
use crate::env::{ git_branch, git_email, git_key_path };
use crate::logging::*;

fn get_remote<'a>(repo: &'a Repository, meta: &Metadata) -> Result<Remote<'a>, Response> {
    let meta = meta.clone();
    
    let Ok(origins) = repo.remotes() else {
        error!("Couldn't find remotes for repo.");
        return Err(Response::ise("Failed to find remotes", meta));
    };
    let Some(origin_str_bytes) = origins.iter_bytes().next() else {
        error!("Couldn't find origin remote for repo.");
        return Err(Response::ise("Failed to find origin remote", meta));
    };
    let origin_str = String::from_utf8_lossy(origin_str_bytes);
    let Ok(origin) = repo.find_remote(&origin_str) else {
        error!("Failed to find origin remote");
        return Err(Response::ise("Failed to find origin remote", meta));
    };
    
    Ok(origin)
}

fn get_branch_refspec(repo: &Repository, meta: &Metadata) -> Result<String, Response> {
    // Push commit to remote
    let Ok(branch) = repo.find_branch(git_branch(), git2::BranchType::Local) else {
        error!("Failed to find local branch {}", git_branch());
        return Err(Response::ise(&format!("Failed to find local branch {}", git_branch()), meta.clone()));
    };
    let branch_ref = branch.into_reference();

    let branch_ref_name = branch_ref
        .name()
        .map(Into::into)
        .unwrap_or_else(|| String::from_utf8_lossy(branch_ref.name_bytes()))
        .to_string();

    Ok(branch_ref_name)
}

fn get_auth_callbacks() -> git2::RemoteCallbacks<'static> {
    use git2::{ RemoteCallbacks, Cred, CredentialType };

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, username_from_url, cred_type| {
        let username = username_from_url.unwrap_or("git");
        if !cred_type.contains(CredentialType::SSH_KEY) {
            error!("SSH KEYS ARE NOT SUPPORTED (supported: {:?})", cred_type.iter_names().collect::<Vec<_>>());
            return Err(git2::Error::from_str("Repository remote doesn't support SSH keys"));
        }
        let cred_res = Cred::ssh_key(
            username,
            None,
            Path::new(git_key_path()),
            None,
        );
        cred_res
    });
    callbacks
}

pub fn ensure_repo_up_to_date(repo_path: &Path, meta: &Metadata) -> Result<(), Response> {

    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::ise("Failed to open repository", meta));
    };
    let Ok(head) = repo.head() else {
        error!("Failed to get head");
        return Err(Response::ise("Failed to get head", meta));
    };
    let Ok(head) = head.peel(ObjectType::Commit) else {
        error!("Failed to get head object");
        return Err(Response::ise("Failed to get head object", meta));
    };



    // Reset repo
    if let Err(e) = repo.reset(&head, ResetType::Hard, None) {
        error!("Failed to get reset repo");
        return Err(Response::ise(&format!("Failed to reset repo: {e:?}"), meta));
    }
    trace!("Successfully reset repo");

    // Fetch repo
    let mut remote = get_remote(&repo, &meta)?;

    if let Err(e) = remote.download(&[] as &[&str], Some(FetchOptions::new().remote_callbacks(get_auth_callbacks()))) {
        error!("Failed to download remote: {e:?}");
        return Err(Response::ise(&format!("Failed to download remote: {e:?}"), meta));
    }
    if let Err(e) = remote.disconnect() {
        error!("Failed to disconnect remote: {e:?}");
        return Err(Response::ise(&format!("Failed to disconnect remote: {e:?}"), meta));
    }

    if let Err(e) = remote.update_tips(Some(&mut get_auth_callbacks()), true, AutotagOption::Unspecified, None) {
        error!("Failed to update remote tips: {e:?}");
        return Err(Response::ise(&format!("Failed to update remote tips: {e:?}"), meta));
    }
    trace!("Successfully fetched new repo commits successfully");



    // Fast forward repo
    let Ok(fetch_head) = repo.find_reference("FETCH_HEAD") else {
        error!("Failed to find FETCH_HEAD after fetching");
        return Err(Response::ise("Failed to find FETCH_HEAD", meta));
    };
    let Ok(fetch_commit) = repo.reference_to_annotated_commit(&fetch_head) else {
        error!("Failed to convert FETCH_HEAD to annotated commit");
        return Err(Response::ise("Failed to convert FETCH_HEAD to annotated commit", meta));
    };
    let ref_path = format!("refs/heads/{}", git_branch());
    let Ok(mut local_branch) = repo.find_reference(&ref_path) else {
        error!("Failed to find {} branch local ref", git_branch());
        return Err(Response::ise(&format!("Failed to find {} branch local ref", git_branch()), meta));
    };

    let local_branch_name = local_branch
        .name()
        .map(Into::into)
        .unwrap_or_else(|| String::from_utf8_lossy(local_branch.name_bytes()))
        .to_string();

    let msg = format!("Fast-Forward: Setting {local_branch_name} to id: {}", fetch_commit.id());
    if let Err(e) = local_branch.set_target(fetch_commit.id(), &msg) {
        error!("Failed to set branch `{}` to FETCH_HEAD: {e:?}", git_branch());
        return Err(Response::ise(&format!("Failed to set branch `{}` to FETCH_HEAD: {e:?}", git_branch()), meta));
    }
    if let Err(e) = repo.set_head(&local_branch_name) {
        error!("Failed to set HEAD to branch `{}`: {e:?}", git_branch());
        return Err(Response::ise(&format!("Failed to set HEAD to branch `{}`: {e:?}", git_branch()), meta));
    }
    if let Err(e) = repo.checkout_head(Some(CheckoutBuilder::default().force())) {
        error!("Failed to checkout repo branch `{}`: {e:?}", git_branch());
        return Err(Response::ise(&format!("Failed to checkout branch `{}`: {e:?}", git_branch()), meta));
    }
    trace!("Successfully fast-forwarded new commits (pulled)");

    Ok(())
}

pub fn make_repo_commit(repo_path: &Path, file_to_add: &Path, meta: &Metadata) -> Result<(), Response> {

    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::ise("Failed to open repository", meta));
    };
    let Ok(mut index) = repo.index() else {
        error!("Failed to get current index for the repo");
        return Err(Response::ise("Failed to get repo index", meta));
    };

    // Stage changes
    if let Err(e) = index.add_path(file_to_add) {
        error!("Couldn't add modified file @ {file_to_add:?} to index");
        return Err(Response::ise(&format!("Failed to stage file changes: {e:?}"), meta));
    }
    if let Err(e) = index.write() {
        error!("Failed to write index with saved file changes");
        return Err(Response::ise(&format!("Failed to save index: {e:?}"), meta));
    }
    let Ok(tree_id) = index.write_tree() else {
        error!("Failed to write tree with saved file changes");
        return Err(Response::ise("Failed to write tree with staged changes", meta));
    };


    // Commit staged changes
    let Ok(head) = repo.head() else {
        error!("Failed to HEAD ref");
        return Err(Response::ise("Failed to get head", meta));
    };
    let Some(head_target) = head.target() else {
        error!("Failed to HEAD OID");
        return Err(Response::ise("Failed to get head target", meta));
    };
    let Ok(head_commit) = repo.find_commit(head_target) else {
        error!("Failed to get commit struct corresponding to HEAD");
        return Err(Response::ise("Failed to get head commit", meta));
    };
    let Ok(tree) = repo.find_tree(tree_id) else {
        error!("Failed to get tree struct corresponding to staged changes");
        return Err(Response::ise("Failed to get tree with staged changes", meta));
    };

    let Ok(sig) = Signature::now("ARCS Admin Panel", git_email()) else {
        error!("Failed to create signature for ARCS Admin Panel commit");
        return Err(Response::ise("Failed to create signature", meta));
    };
    let Some(updated_chall) = file_to_add.parent().and_then(|p| p.file_name()).and_then(|f| f.to_str()) else {
        error!("Couldn't get chall name for file @ {file_to_add:?}");
        return Err(Response::ise("Failed to get challenge name for commit message", meta));
    };

    let message = format!("`ADMIN_PANEL_MANAGEMENT:` updated chall.yaml for challenge `{updated_chall}`");

    debug!("commit message: {message:?}");

    if repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &message, 
        &tree,
        &[&head_commit],
    ).is_err() {
        return Err(Response::ise("Failed to commit changes", meta));
    }


    // Push commit to remote
    let mut remote = get_remote(&repo, &meta)?;
    let branch_refspec = get_branch_refspec(&repo, &meta)?;

    if let Err(e) = remote.push::<&str>(&[&branch_refspec], Some(PushOptions::new().remote_callbacks(get_auth_callbacks()))) {
        error!("Failed to push to remote: {e:?}");
        return Err(Response::ise(&format!("Failed to push to remote: {e:?}"), meta));
    }


    Ok(())
}
