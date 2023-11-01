use std::path::Path;

use git2::{
    Repository,
    Oid,
    Object, ObjectType,
    PushOptions
};

use super::GitResult;
use super::get_branch_refspec;
use super::stage::stage_all_unstaged;
use super::remote::get_remote;
use super::signature_auth::get_auth_callbacks;
use super::commit::{ commit_tree, new_commit_from_files };

use crate::logging::*;
use crate::server::{ Response, Metadata };

pub fn prepare_repo_commit_all(repo: &Repository) -> GitResult<Option<Oid>> {
    // Stage and commit all unstaged changes
    if let Some((tree_id, message)) = stage_all_unstaged(repo)? {
        commit_tree(repo, tree_id, &message).map(Some)
    } else {
        Ok(None)
    }
}

pub fn get_ref_log_save(repo: &Repository) -> GitResult<Object> {
    let head_ref = repo.head()?;
    debug!("HEAD ref: {:?}", head_ref.peel_to_commit()?.id());
    head_ref.peel(ObjectType::Commit)
}

pub fn hard_reset_to_ref_log(repo: &Repository, ref_object: Object) -> GitResult {
    repo.reset(&ref_object, git2::ResetType::Hard, None)
}

pub fn make_commit(repo_path: &Path, files_to_add: &[&Path], message: &str, meta: &Metadata) -> Result<(), Response> {
    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::git_err(meta, "Failed to open repository"));
    };
    trace!("Opened repository");

    if let Err(e) = new_commit_from_files(&repo, files_to_add, message) {
        error!("Failed to commit files: {}", e);
        Err(Response::git_err(meta, &format!("Failed to commit files: {}", e)))
    } else {
        Ok(())
    }
}

pub fn push_all(repo_path: &Path, meta: &Metadata) -> Result<(), Response> {
    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::git_err(meta, "Failed to open repository"));
    };
    trace!("Opened repository");


    // Push commit to remote
    let Ok(mut remote) = get_remote(&repo) else {
        error!("Failed to get remote");
        return Err(Response::git_err(meta, "Failed to get remote"));
    };
    let branch_refspec = get_branch_refspec(&repo, &meta)?;

    if let Err(e) = remote.push::<&str>(&[&branch_refspec], Some(PushOptions::new().remote_callbacks(get_auth_callbacks()))) {
        error!("Failed to push to remote: {e:?}");
        Err(Response::git_err(meta, &format!("Failed to push to remote: {e:?}")))
    } else {
        Ok(())
    }
}
