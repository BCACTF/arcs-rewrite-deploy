pub mod structs;

use serde_yaml::Value as YamlValue;

use crate::structs::{ValueType, get_type};

pub trait StrList
where Self: Sized {
    type Error; 
    fn from_iter<'a>(iter: impl Iterator<Item = &'a str>) -> Result<Self, Self::Error>;

    fn from_value_mismatch(iter: impl Iterator<Item = ValueType>) -> Self::Error;

    fn not_sequence(type_enum: ValueType) -> Self::Error;
}

pub fn as_str_list<T: StrList>(value: &YamlValue) -> Result<T, T::Error> {

    if !value.is_sequence() {
        return Err(T::not_sequence(get_type(value)));
    }


    if let Some(sequence) = value.as_sequence() {
        let mut strings = vec![];
        let mut bad_type = vec![];

        sequence.iter().for_each(
            |val| if let Some(name) = val.as_str() {
                strings.push(name);
            } else {
                bad_type.push(get_type(val));
            }
        );

        if bad_type.is_empty() {
            T::from_iter(strings.into_iter())
        } else {
            Err(T::from_value_mismatch(bad_type.into_iter()))
        }
    } else { unreachable!() }
}
