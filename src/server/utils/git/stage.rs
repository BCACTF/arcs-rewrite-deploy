use crate::logging::*;

use git2::{
    Repository,
    DiffOptions,
    IndexAddOption,
    Oid,
    Error
};


pub type GitResult<T = ()> = Result<T, Error>;

pub fn stage_all_unstaged(repo: &Repository) -> GitResult<Option<(Oid, String)>> {
    let unstaged_diff = repo.diff_index_to_workdir(
        None,
        Some(
            DiffOptions::new()
                .show_unmodified(false)
                .show_untracked_content(true)
                .recurse_untracked_dirs(true)
                .include_ignored(false)
        ),
    )?;
    trace!("Got unstaged/untracked diff");

    let diff_files: Vec<&[u8]> = unstaged_diff
        .deltas()
        .flat_map(|d| {
            let old_file = d.old_file().path_bytes();
            let new_file = d.new_file().path_bytes();
            if old_file == new_file {
                [new_file, None]
            } else {
                [old_file, new_file]
            }            
        })
        .flatten()
        .collect();

    if diff_files.is_empty() {
        debug!("No unstaged/untracked files to add");
        return Ok(None);
    }


    let diff_files_readable = diff_files
        .iter()
        .copied()
        .map(String::from_utf8_lossy)
        .collect::<Vec<_>>();

    debug!("Unstaged/untracked files to add: {diff_files_readable:?}");

    let mut index = repo.index()?;
    trace!("Got current index");
    
    index.add_all(&diff_files, IndexAddOption::DEFAULT, None)?;
    trace!("Modified index to add unstaged/untracked files");

    index.write()?;
    trace!("Wrote index to disk");

    let tree_oid = index.write_tree()?;
    trace!("Wrote index tree (id {tree_oid}) to disk (for committing)");


    use std::fmt::Write;

    let mut commit_message = String::new();
    let _ = writeln!(&mut commit_message, "ADMIN_PANEL_MANAGEMENT: Committed local changes (see desc for details)");
    let _ = writeln!(&mut commit_message);
    let _ = writeln!(&mut commit_message, "Changed/added files:");
    for file in diff_files_readable {
        let _ = writeln!(&mut commit_message, " - {file:?}");
    }

    debug!("Generated commit message: {commit_message}");

    Ok(Some((tree_oid, commit_message)))
}