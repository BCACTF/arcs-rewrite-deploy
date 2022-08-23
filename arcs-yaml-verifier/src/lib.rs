use serde::{Serialize, Deserialize};
use serde_yaml::Error as YamlError;

use std::fmt::Display;
use std::collections::HashSet;

use lazy_static::lazy_static;

#[allow(unused)]
static CATEGORY_DEFAULTS: [&str; 5] = ["misc", "binex", "foren", "crypto", "webex"];

lazy_static! {
    static ref CATEGORIES: Box<[&'static str]> = Box::new([]);
}

pub enum Category {

}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct YamlShape {
    name: String,
    categories: HashSet<String>,
}

impl Display for YamlShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", self))
    }
}

pub fn verify_yaml(yaml_text: &str) -> Result<YamlShape, YamlError> {
    serde_yaml::from_str(yaml_text)
}


