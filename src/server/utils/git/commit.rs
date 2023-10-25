use crate::logging::*;
use super::GitResult;
use super::signature_auth::get_signature;

use git2::{ Repository, Oid, Error };
use git2::{ ErrorCode::NotFound, ErrorClass::Reference };

pub fn commit_tree(repo: &Repository, tree_id: Oid, message: &str) -> GitResult<Oid> {
    // Commit staged changes
    let head = repo.head()?;
    trace!("Got HEAD ref");

    let head_target = head.target().ok_or(Error::new(NotFound, Reference, "Failed to get HEAD target"))?;
    trace!("Got HEAD target OID");

    let head_commit = repo.find_commit(head_target)?;
    trace!("Got commit struct for HEAD");



    let new_tree = repo.find_tree(tree_id)?;
    trace!("Got tree struct for staged changes");

    let commit_sig = get_signature()?;
    trace!("Got signature for ARCS Admin Panel commit");

    repo.commit(
        Some("HEAD"),
        &commit_sig,
        &commit_sig,
        message, 
        &new_tree,
        &[&head_commit],
    )
}

pub fn new_commit_from_files(repo: &Repository, files_to_add: &[&std::path::Path], message: &str) -> GitResult<Oid> {
    let mut index = repo.index()?;
    trace!("Got index");

    // Stage changes
    for file in files_to_add {
        trace!("Adding file @ {file:?} to index");
        index.add_path(file)?;
    }
    trace!("Staged all files");

    index.write()?;
    trace!("Wrote index");

    let tree_id = index.write_tree()?;
    trace!("Wrote tree");

    commit_tree(repo, tree_id, message)
}
