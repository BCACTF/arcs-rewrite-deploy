use std::path::{Path, PathBuf};

use crate::{YamlShape, YamlVerifyError, YamlCorrectness, categories::structs::Category};


impl YamlShape {
    pub fn try_from_str(s: &str, correctness: &YamlCorrectness) -> Result<YamlShape, YamlVerifyError> {
        super::verify_yaml(s, Some(correctness.clone()))
    }
}
impl YamlShape {
    pub fn file_iter(&self) -> Option<impl Iterator<Item = &Path>> {
        self.files.as_ref().map(crate::files::Files::iter)
    }
    pub fn files(&self) -> Option<&[PathBuf]> {
        self.files.as_ref().map(crate::files::Files::slice)
    }

    pub fn author_iter(&self) -> impl Iterator<Item = &str> {
        self.authors.iter()
    }
    pub fn authors(&self) -> &[String] {
        self.authors.slice()
    }

    pub fn category_str_iter(&self) -> impl Iterator<Item = &str> {
        self.categories.iter().map(Category::as_str)
    }
    pub fn category_iter(&self) -> impl Iterator<Item = &Category> {
        self.categories.iter()
    }
    pub fn categories(&self) -> &[Category] {
        self.categories.slice()
    }

    pub fn hint_iter(&self) -> impl Iterator<Item = &str> {
        self.hints.iter()
    }
    pub fn hints(&self) -> &[String] {
        self.hints.slice()
    }
}

impl YamlShape {
    pub fn flag_str(&self) -> Option<&str> {
        if let crate::flag::Flag::String(s) = &self.flag {
            Some(&s)
        } else { None }
    }
    pub fn flag_filepath(&self) -> Option<&Path> {
        if let crate::flag::Flag::File(p) = &self.flag {
            Some(&p)
        } else { None }
    }
}

impl YamlShape {
    pub fn chall_name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
    
    pub fn points(&self) -> u64 { self.points }

    pub fn visible(&self) -> bool { self.visible }
}

