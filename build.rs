use std::os::unix::prelude::OsStrExt;
use std::fs::{File, self};
use std::io::prelude::*;

use std::collections::HashSet;

use walkdir::WalkDir;

fn main() {
    let mut filenames = HashSet::new();

    for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
        let matches = entry.file_name().as_bytes() == ".required_envs".as_bytes();
        
        if matches {
            filenames.insert(entry.into_path());
        }
    }

    let (file_strings, file_errors): (Vec<_>, Vec<_>) = filenames
        .iter()
        .map(fs::read_to_string)
        .partition(Result::is_ok);

    let file_strings: Vec<_> = file_strings
        .into_iter()
        .filter_map(Result::ok)
        .collect();

    let file_errors: Vec<_> = file_errors
        .into_iter()
        .filter_map(Result::err)
        .collect();

    if !file_errors.is_empty() {
        file_errors
            .into_iter()
            .for_each(|error| eprintln!("{}", error));
        
        panic!("Some `.required_envs` files failed to read at compile time. Please see above errors for more information.");
    }


    let required_vars: HashSet<_> = file_strings
        .iter()
        .flat_map(|string_ref| string_ref.lines())
        .map(str::trim)
        .filter(|trimmed_str| !trimmed_str.is_empty())
        .collect();
        
    let required_envs_string = required_vars
        .into_iter()
        .fold(String::new(), |mut prev, new_var| {
            prev.push_str(new_var);
            prev.push('\n');
            prev
        });

    File::create(".required_envs_conglomerate").unwrap().write_all(required_envs_string.as_bytes()).unwrap();
}
