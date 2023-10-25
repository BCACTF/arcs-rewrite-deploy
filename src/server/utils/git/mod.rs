mod signature_auth;
mod remote;

mod stage;
mod commit;
mod fetch;
mod merge;

mod prep;

use std::path::Path;
use git2::{Repository, Signature, PushOptions};

use crate::server::responses::{Metadata, Response};
use crate::env::{ git_branch, git_email };
use crate::logging::*;
use crate::server::utils::git::prep::prepare_repo_commit_all;

pub use prep::{ make_commit, push_all };

pub type GitResult<T = ()> = Result<T, git2::Error>;



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

pub fn ensure_repo_up_to_date(repo_path: &Path, meta: &Metadata) -> Result<bool, Response> {
    let meta = meta.clone();

    let Ok(repo) = Repository::open(repo_path) else {
        error!("Failed to open repository");
        return Err(Response::ise("Failed to open repository", meta));
    };
    trace!("Opened repository");


    let Ok(commit_oid_opt) = prepare_repo_commit_all(&repo) else {
        error!("Failed to commit all unstaged changes");
        return Err(Response::ise("Failed to commit all unstaged changes", meta));
    };
    match commit_oid_opt {
        Some(commit_oid) => debug!("Commit OID: {commit_oid}"),
        None => debug!("No commit created"),
    }


    let Ok(saved_ref_log) = prep::get_ref_log_save(&repo) else {
        error!("Failed to get ref log object for rollback");
        return Err(Response::ise("Failed to get ref log object for rollback", meta));
    };
    trace!("Saved ref log for rollback");


    let could_connect = if let Some(mut remote) = remote::try_get_connected_remote(&repo).unwrap() {
        if let Err(e) = fetch::fetch_from_remote(&mut remote) {
            error!("Failed to fetch from remote: {e:?}");
            return Err(Response::ise(&format!("Failed to fetch from remote: {e:?}"), meta));
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
                    return Err(Response::ise(&format!("Failed to hard reset to ref log: {e:?}"), meta));
                }

                return Err(Response::success(meta, Some(serde_json::json!{{ "applied": false }})));
            },
            Err(e) => {
                error!("Failed to merge fetched commits: {e:?}");
                if let Err(e) = prep::hard_reset_to_ref_log(&repo, saved_ref_log) {
                    error!("Failed to hard reset to ref log: {e:?}");
                    return Err(Response::ise(&format!("Failed to merge fetched commits: {e:?}.\nWhile rolling back, encountered an error: {e:?}"), meta));
                } else {
                    return Err(Response::ise(&format!("Failed to merge fetched commits: {e:?}"), meta));
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


