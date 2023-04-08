use std::fmt::Debug;

#[allow(unused)]
static CATEGORY_DEFAULTS: [&str; 5] = ["misc", "binex", "foren", "crypto", "webex"];

use lazy_static::lazy_static;
use std::collections::HashSet;
lazy_static!{
    static ref CATEGORY_RAW_ENV_VAR: Option<String> = std::env::var("CATEGORIES").ok();

    pub static ref CATEGORIES: HashSet<String> = CATEGORY_RAW_ENV_VAR.as_ref().map_or_else(
        || CATEGORY_DEFAULTS.into_iter().map(str::to_string).collect(),
        |val| val.split(',').map(|part| part.trim().to_string()).collect(),
    );
}

#[derive(PartialEq)]
pub struct Category {
    name: String,
}

impl Category {
    pub fn try_new(name: &'_ str) -> Option<Self> {
        // CATEGORIES.get(name).map(|name| Self { name })
        Some(Self { name: name.to_string() })
    }
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl Debug for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Category< {} >", self.name)
    }
}

#[derive(PartialEq)]
pub struct Categories(Vec<Category>);

impl Categories {
    pub fn try_new<'a>(category_names: impl IntoIterator<Item = &'a str>) -> Result<Categories, Vec<&'a str>> {
        let mut good_cats = vec![];
        let mut invalid_cat_names = vec![];

        for name in category_names {
            if let Some(cat) = Category::try_new(name) {
                good_cats.push(cat);
            } else {
                invalid_cat_names.push(name);
            }
        }

        if invalid_cat_names.is_empty() {
            Ok(Self(good_cats))
        } else {
            Err(invalid_cat_names)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Category> {
        self.0.iter()
    }

    pub fn slice(&self) -> &[Category] {
        &self.0
    }
}

impl Debug for Categories {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Categories< ")?;
        if let Some(cat) = self.0.first() {
            write!(f, "{}", cat.name)?;

            for cat in self.0.iter().skip(1) {
                write!(f, ", {}", cat.name)?;
            }
        }
        write!(f, " >")
    }
}

