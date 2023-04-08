use std::{fmt::{Display, Debug}, path::PathBuf, str::FromStr};


use serde_yaml::Value as YamlValue;

use crate::structs::{get_type, ValueType};


pub fn file_list(value: &YamlValue) -> Result<Files, FileErrors> {
    let sequence = value.as_sequence().ok_or_else(|| FileErrors::BadBaseType(get_type(value)))?;

    let entries = sequence
        .iter().enumerate()
        .map(
            |(idx, val)| val
                .as_mapping()
                .ok_or_else(|| FileEntryError::ItemNotMapping(idx, get_type(val)))?
                .get("src")
                .ok_or_else(|| FileEntryError::SrcKeyNotDefined(idx))
        ).enumerate()
        .map(|(idx, res)| {
            let value = res?;
            value.as_str()
                .ok_or_else(|| FileEntryError::SrcPathNotString(idx, get_type(value)))
        }).enumerate()
        .map(|(idx, value)| {
            let str_slice = value?;
            PathBuf::from_str(str_slice)
                .map_err(|_| FileEntryError::InvalidPath(idx, str_slice.to_string()))
        });

    let mut paths = vec![];
    let mut errs = vec![];

    entries.for_each(
        |res| match res {
            Ok(path) => paths.push(path),
            Err(e) => errs.push(e),
        }
    );

    if errs.is_empty() {
        Ok(Files(paths))
    } else {
        Err(FileErrors::EntryErrors(errs))
    }
}


#[derive(Clone, PartialEq)]
pub struct Files(Vec<PathBuf>);

#[derive(Debug, Clone, PartialEq)]
pub enum FileEntryError {
    ItemNotMapping(usize, ValueType),
    SrcKeyNotDefined(usize),
    SrcPathNotString(usize, ValueType),
    InvalidPath(usize, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileErrors {
    BadBaseType(ValueType),
    EntryErrors(Vec<FileEntryError>),
}

impl Display for FileEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FileEntryError::*;
        match self {
            ItemNotMapping(i, t) => write!(f, "#{i}: Must be in the format of `- src: <file>`, not {t}."),
            SrcKeyNotDefined(i) => write!(f, "#{i}: Doesn't have a `src:` in it."),
            SrcPathNotString(i, t) => write!(f, "#{i}: src must be a filepath, not {t}."),
            InvalidPath(i, bad_path) => write!(f, "#{i}: src \"{bad_path}\" is an invalid filepath."),
        }
    }
}
impl Display for FileErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FileErrors::*;
        match self {
            BadBaseType(t) => write!(f, "Files should be a list, not {t}."),
            EntryErrors(errs) => {
                writeln!(f, "Some entries under `files` are invalid:")?;
                for err in errs {
                    writeln!(f, "        {err}")?;
                }
                Ok(())
            },
        }
    }
}

impl Debug for Files {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Files ")?;
        f.debug_list()
            .entries(self.0.iter())
            .finish()
    }
}

pub trait Flop {
    type Target;
    fn flop(self) -> Self::Target;
}
impl<T, E> Flop for Option<Result<T, E>> {
    type Target = Result<Option<T>, E>;
    fn flop(self) -> Self::Target {
        if let Some(res) = self {
            res.map(Some)
        } else { Ok(None) }
    }
}
