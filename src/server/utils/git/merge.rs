use std::collections::{HashSet, HashMap};

use crate::logging::*;
use super::{
    GitResult,
    signature_auth::get_signature,
};

use git2::{
    Repository,
    Index, AnnotatedCommit,
    Reference, build::CheckoutBuilder,
};

pub fn fast_forward(repo: &Repository, local_head: &mut Reference, remote_commit: &AnnotatedCommit) -> GitResult {
    let local_branch_name = String::from_utf8_lossy(local_head.name_bytes()).to_string();

    let remote_commit_id = remote_commit.id();

    let ref_message = format!("Fast-forwarding branch `{local_branch_name}` to id: {remote_commit_id}");
    debug!("{ref_message}");


    local_head.set_target(remote_commit_id, &ref_message)?;
    trace!("Updated the HEAD target to the remote commit");

    repo.set_head(&local_branch_name)?;
    trace!("Set HEAD to mean the new remote commit");

    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    trace!("Applied changes in HEAD into the working directory");

    Ok(())
}


// TODO: Add more logging
fn merge_disjointed(
    repo: &Repository,
    local_commit: &AnnotatedCommit,
    remote_commit: &AnnotatedCommit,
    prioritize_remote: bool,
) -> GitResult<bool> {
    let local_refname = String::from_utf8_lossy(local_commit.refname_bytes());
    let remote_refname = String::from_utf8_lossy(remote_commit.refname_bytes());
    
    let local_branch_name = local_refname.trim_start_matches("refs/heads/");
    let remote_branch_name = remote_refname.trim_start_matches("refs/heads/");
    debug!("remote: {remote_branch_name:?}, local: {local_branch_name:?}");
    
    let local_tree = repo.find_commit(local_commit.id())?.tree()?;
    let remote_tree = repo.find_commit(remote_commit.id())?.tree()?;

    let mut remote_index = Index::new()?;
    remote_index.read_tree(&remote_tree)?;
    trace!("Got remote index in case of conflicts");
    
    let ancestor_tree = repo.find_commit(repo.merge_base(local_commit.id(), remote_commit.id())?)?.tree()?;
    trace!("Got base commit");

    let mut ancestor_index = Index::new()?;
    ancestor_index.read_tree(&ancestor_tree)?;

    let mut index = repo.merge_trees(&ancestor_tree, &local_tree, &remote_tree, None)?;
    trace!("Merged trees");

    let conflicts = if index.has_conflicts() {
        trace!("Repo has conflicts");

        if !prioritize_remote {
            trace!("No behavior specified for conflicts, exiting prematurely");
            return Ok(false);
        }

        trace!("Resolving conflicts, prioritizing remote changes");

        let mut conflicts = HashSet::new();

        for conflict in index.conflicts()? {
            let conflict = conflict?;
            if let Some(index_entry) = conflict.our {
                let path = String::from_utf8_lossy(&index_entry.path).to_string();
                debug!("Resolved conflict in favor of remote: {path}");
                conflicts.insert(path);
            }
        }

        for path in &conflicts {
            index.remove_path(std::path::Path::new(&path))?;
            
            if let Some(entry) = remote_index.get_path(std::path::Path::new(&path), 0) {
                debug!("{entry:?} (stage {})", (entry.flags >> 12) & 0b11);
                index.add(&entry)?;
            } else {
                return Err(git2::Error::from_str("Failed to find conflicting index in remote repository"));
            }
        }

        conflicts.into_iter().collect()
    } else { vec![] };

    trace!("Resolved conflicts");

    let tree_id = index.write_tree_to(repo)?;
    let commit_tree = repo.find_tree(tree_id)?;
    trace!("Got tree for commit");

    let local_commit = repo.find_commit(local_commit.id())?;
    let remote_commit = repo.find_commit(remote_commit.id())?;
    trace!("Got commit objects for local and remote");
    debug!("local id: {}, remote id: {}", local_commit.id(), remote_commit.id());


    use std::fmt::Write;
    let mut commit_message = format!(
        "ADMIN_PANEL_MANAGEMENT: Merged remote {} ({}) into local {} ({})",
        local_branch_name,
        local_commit.id(),
        remote_branch_name,
        remote_commit.id(),
    );
    let _ = writeln!(&mut commit_message);
    let _ = writeln!(&mut commit_message, "Files with conflicts overwritten:");
    for conflict in conflicts {
        let _ = writeln!(&mut commit_message, " - {}", conflict);
    }


    let signature = get_signature()?;


    let _merge_commit = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &commit_message,
        &commit_tree,
        &[&local_commit, &remote_commit],
    )?;

    repo.checkout_head(None)?;

    Ok(true)
}


pub fn merge_fetched(repo: &Repository, prioritize_remote: bool) -> GitResult<bool> {
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    trace!("Got FETCH_HEAD ref");
    
    let remote_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    trace!("Got annotated commit for the fetch head");
    
    let analysis = repo.merge_analysis(&[&remote_commit])?;

    if analysis.0.is_up_to_date() {
        trace!("Repo is up to date");
        Ok(true)
    } else if analysis.0.is_fast_forward() {
        trace!("Repo is fast-forwardable");
        fast_forward(repo, &mut repo.head()?, &remote_commit)?;
        Ok(true)
    } else {
        trace!("Repo is disjointed (there are commits on remote that aren't on local and vice versa)");
        merge_disjointed(
            repo,
            &repo.reference_to_annotated_commit(&repo.head()?)?,
            &remote_commit,
            prioritize_remote,
        )
    }
}
