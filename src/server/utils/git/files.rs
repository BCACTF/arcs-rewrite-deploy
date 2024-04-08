use crate::logging::*;
use git2::Repository;

use super::GitResult;

pub fn get_all_chall_names(repo: &Repository) -> GitResult<Vec<String>> {
    let tree = repo.head()?.peel_to_tree()?;

    let mut paths = Vec::new();

    tree.walk(
        git2::TreeWalkMode::PreOrder,
        |root, entry| {
            if entry.name_bytes() == b"chall.yaml" && !root.is_empty() {
                let name = root.trim_matches('/');
                paths.push(name.to_string());
            }

            git2::TreeWalkResult::Ok
        },
    )?;
    debug!("Found challs: {paths:?}");

    Ok(paths)
}
