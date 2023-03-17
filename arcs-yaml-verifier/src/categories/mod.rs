pub mod structs;

use std::fmt::Display;

pub use structs::Categories;

use serde_yaml::Value as YamlValue;

use crate::structs::{get_type, ValueType};

// use guard::guard;

#[derive(Debug, Clone)]
pub enum CategoryError {
    InvalidCategories(Vec<String>, Vec<ValueType>),
    InvalidBaseType(ValueType),
    MissingKey,
}
impl Display for CategoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CategoryError::*;
        match self {
            InvalidCategories(names, types) => {
                if !names.is_empty() {
                    if names.len() == 1 {
                        write!(f, "`{}` is not a valid category name.", names[0])?;
                    } else {
                        write!(f, "`{names:?}` are not valid category names.")?;
                    }
                }
                if !types.is_empty() {
                    write!(f, "Category names must also be strings.")?;
                }
            }
            InvalidBaseType(t) => write!(f, "Categories should be a list, not {t}.")?,
            MissingKey => write!(f, "You have to define `categories`.")?,
        }
        Ok(())
    }
}

pub fn value_to_categories(value: &YamlValue) -> Result<Categories, CategoryError> {
    use CategoryError::*;

    if !value.is_sequence() {
        return Err(InvalidBaseType(get_type(value)));
    }

    if let Some(sequence) = value.as_sequence() {
        let mut cand_name = vec![];
        let mut bad_type = vec![];

        sequence.iter().for_each(
            |val| if let Some(name) = val.as_str() {
                cand_name.push(name);
            } else {
                bad_type.push(get_type(val));
            }
        );

        match Categories::try_new(cand_name) {
            Ok(categories) => if bad_type.len() > 0 {
                Err(InvalidCategories(Vec::new(), bad_type))
            } else {
                Ok(categories)
            },
            Err(bad_names) => Err(InvalidCategories(bad_names.into_iter().map(str::to_string).collect(), bad_type))
        }
    } else { unreachable!() }

}
