use std::{fmt::{Display, Debug}, path::PathBuf, str::FromStr};

use serde_yaml::Value as YamlValue;

use crate::structs::{get_type, ValueType};

pub fn get_flag(value: &YamlValue) -> Result<Flag, FlagError> {
    if let Some(flag_str) = value.as_str() {
        Ok(Flag::String(flag_str.to_string()))
    } else if let Some(mapping) = value.as_mapping() {
        if let Some(Some(file)) = mapping.get("file").map(YamlValue::as_str) {
            if let Ok(path) = PathBuf::from_str(file) {
                Ok(Flag::File(path))
            } else {
                Err(FlagError::BadPath(file.to_string()))
            }
        } else {
            Err(FlagError::MappingNeedsFile)
        }
    } else {
        Err(FlagError::BadType(get_type(value)))
    }
}

#[derive(Clone, PartialEq)]
pub enum Flag {
    String(String),
    File(PathBuf),
}

impl Debug for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Flag::{ String, File };
        match self {
            String(s) => write!(f, "Flag< {s} >"),
            File(p) => write!(f, "File< @ {} >", p.display()),
        }
    }
}


#[derive(Debug, Clone, )]
pub enum FlagError {
    BadType(ValueType),
    
    BadString(String),

    BadPath(String),
    MappingNeedsFile,
    
    MissingKey,
}

impl Display for FlagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FlagError::*;
        match self {
            &BadType(t) => write!(f, "Flag should be a list, not {t}."),
            BadString(s) => write!(f, "The string {s} is not a valid flag."),
            BadPath(p) => write!(f, "The string {p} is not a valid path. (hint: If you want to define a flag with a string, use `flag: <input>`)"),
            &MappingNeedsFile => write!(f, "If you are going to define a flag via a file, you need to have `file: <path>` as an entry under `flag`. (<path> must be a string)"),
            &MissingKey => write!(f, "You have to define `categories`."),
        }
    }
}
