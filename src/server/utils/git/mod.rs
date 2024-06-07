mod signature_auth;
mod remote;
mod files;

mod stage;
mod commit;
mod fetch;
mod merge;

mod prep;

use std::path::Path;
use git2::Repository;

use crate::server::responses::{Metadata, Response};
use crate::env::git_branch;
use crate::logging::*;
use crate::server::utils::git::prep::prepare_repo_commit_all;

pub use prep::{ make_commit, push_all };

pub type GitResult<T = ()> = Result<T, git2::Error>;



fn get_branch_refspec(repo: &Repository, meta: &Metadata) -> Result<String, Response> {
    // Push commit to remote
    let Ok(branch) = repo.find_branch(git_branch(), git2::BranchType::Local) else {
        error!("Failed to find local branch {}", git_branch());
        return Err(Response::git_err(meta.clone(), format!("Failed to find local branch {}", git_branch())));
    };
    let branch_ref = branch.into_reference();

    let branch_ref_name = branch_ref
        .name()
        .map(Into::into)
        .unwrap_or_else(|| String::from_utf8_lossy(branch_ref.name_bytes()))
        .to_string();

    Ok(branch_ref_name)
}

pub fn ensure_repo_up_to_date(repo_path: &Path, meta: &Metadata) -> Result<bool, Response> {
    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::git_err(meta, "Failed to open repository"));
    };
    trace!("Opened repository");


    let Ok(commit_oid_opt) = prepare_repo_commit_all(&repo) else {
        error!("Failed to commit all unstaged changes");
        return Err(Response::git_err(meta, "Failed to commit all unstaged changes"));
    };
    match commit_oid_opt {
        Some(commit_oid) => debug!("Commit OID: {commit_oid}"),
        None => debug!("No commit created"),
    }


    let Ok(saved_ref_log) = prep::get_ref_log_save(&repo) else {
        error!("Failed to get ref log object for rollback");
        return Err(Response::git_err(meta, "Failed to get ref log object for rollback"));
    };
    trace!("Saved ref log for rollback");


    let could_connect = if let Some(mut remote) = remote::try_get_connected_remote(&repo).unwrap() {
        if let Err(e) = fetch::fetch_from_remote(&mut remote) {
            error!("Failed to fetch from remote: {e:?}");
            return Err(Response::git_err(meta, format!("Failed to fetch from remote: {e:?}")));
        }
        trace!("Successfully fetched new remote commits");

        match merge::merge_fetched(&repo, false) {
            Ok(true) => {
                trace!("Successfully merged fetched commits");
            },
            Ok(false) => {
                trace!("Failed to merge (unresolved conflicts)");
                if let Err(e) = prep::hard_reset_to_ref_log(&repo, saved_ref_log) {
                    error!("Failed to hard reset to ref log: {e:?}");
                    return Err(Response::git_err(meta, format!("Failed to hard reset to ref log: {e:?}")));
                }

                return Err(Response::git_err(meta, "Failed to merge fetched commits: unresolved conflicts"));
            },
            Err(e) => {
                error!("Failed to merge fetched commits: {e:?}");
                if let Err(e) = prep::hard_reset_to_ref_log(&repo, saved_ref_log) {
                    error!("Failed to hard reset to ref log: {e:?}");
                    return Err(Response::git_err(meta, format!("Failed to merge fetched commits: {e:?}.\nWhile rolling back, encountered an error: {e:?}")));
                } else {
                    return Err(Response::git_err(meta, format!("Failed to merge fetched commits: {e:?}")));
                }
            },
        }
        true
    } else {
        trace!("Failed to connect to remote, couldn't fetch");
        false
    };

    Ok(could_connect)
}

static LAST_PULL_TIME: std::sync::Mutex<std::time::SystemTime> = std::sync::Mutex::new(std::time::SystemTime::UNIX_EPOCH);

pub fn get_all_chall_names(repo_path: &Path, meta: &Metadata) -> Result<Vec<String>, Response> {
    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::git_err(meta.clone(), "Failed to open repository"));
    };
    trace!("Opened repository");

    if let Ok(mut lock) = LAST_PULL_TIME.try_lock() {
        if let Ok(elapsed) = lock.elapsed() {
            if elapsed.as_secs() > 60 {
                if ensure_repo_up_to_date(repo_path, meta).is_ok() {
                    *lock = std::time::SystemTime::now();
                }
            }
        }
    }

    let chall_names = match files::get_all_chall_names(&repo) {
        Ok(chall_names) => chall_names,
        Err(e) => {
            error!("Failed to get all chall names: {e:?}");
            return Err(Response::git_err(meta.clone(), format!("Failed to get all chall names: {e:?}")));
        },
    };

    Ok(chall_names)
}

