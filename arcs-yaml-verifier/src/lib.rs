pub mod categories;
pub mod structs;

use serde_yaml::Mapping as YamlMapping;
use serde_yaml::Value as YamlValue;
use structs::ValueType;

use categories::Categories;
use categories::CategoryError;
use structs::{ YamlVerifyError, YamlAttribVerifyError };
use structs::get_type;


#[derive(PartialEq, Debug)]
pub struct YamlShape {
    name: String,
    categories: Categories,
    points: u64,
    description: String,

    // hints: StringList,
    // authors: StringList,

    visible: bool,
}



macro_rules! collect_errors {
    ($($vals:ident),+ $(,)?) => {
        collect_errors!(@impl left: $($vals,)+; good: []; errors: [])
    };
    (@impl left: $val:ident, $($next_vals:ident,)*; good: [$($good_exprs:expr,)*]; errors: [$($err_exprs:expr,)*]) => {
        match &$val {
            Ok(_)  => collect_errors!(@impl left: $($next_vals,)*; good: [$($good_exprs,)* $val.unwrap(),]; errors: [$($err_exprs,)*]),
            Err(_) => collect_errors!(@impl left: $($next_vals,)*; good: [$($good_exprs,)*]; errors: [$($err_exprs,)* $val.unwrap_err(),]),
        }
    };
    (@impl left: ; good: [$($good_exprs:expr,)*]; errors: []) => {
        Ok(($($good_exprs,)*))
    };
    (@impl left: ; good: [$($good_exprs:expr,)*]; errors: [$($err_exprs:expr,)*]) => {
        Err(vec![$($err_exprs,)*])
    };
}


pub fn verify_yaml(yaml_text: &str) -> Result<YamlShape, YamlVerifyError> {
    use YamlVerifyError::*;
    use YamlAttribVerifyError::*;

    let base: YamlValue = serde_yaml::from_str(yaml_text).map_err(Unparsable)?;
    let base: &YamlMapping = if let Some(base) = base.as_mapping() { base } else {
        return Err(BaseNotMap(get_type(&base)))
    };

    let categories = base
        .get("categories")
        .map(categories::value_to_categories)
        .unwrap_or(Err(CategoryError::MissingKey))
        .map_err(Categories);

    let name = if let Some(name_val) = base.get("name") {
        name_val
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| NameNotString(get_type(name_val)))
    } else {
        Err(NameNotString(ValueType::NULL))
    };


    let points = if let Some(point_val) = base.get("value") {
        point_val
            .as_u64()
            .ok_or_else(|| PointsNotInt(get_type(point_val)))
    } else {
        Err(PointsNotInt(ValueType::NULL))
    };


    let description = if let Some(desc_val) = base.get("description") {
        desc_val
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| DescNotString(get_type(desc_val)))
    } else {
        Err(DescNotString(ValueType::NULL))
    };


    let visible = if let Some(point_val) = base.get("visible") {
        point_val
            .as_bool()
            .ok_or_else(|| VisNotBool(get_type(point_val)))
    } else {
        Err(VisNotBool(ValueType::NULL))
    };
    // println!("cat: {categories:#?}, name: {name:#?}");

    let (
        categories,
        name,
        points,
        description,
        visible,
    ) = collect_errors!(
        categories,
        name,
        points,
        description,
        visible,
    ).map_err(PartErrors)?;

    Ok(YamlShape { categories, name, points, description, visible })
}

pub mod yaml_logging_utils {
    use arcs_deploy_shared_structs::shortcuts::*;

    pub fn write_path(target: &mut impl std::io::Write, path: &str) -> IOResult<()> {
        writeln!(target, "{:-^30}", path)
    }
}

#[doc(hidden)]
pub mod __main {

    pub fn main() {
        let mut errors_encountered = false;

        std::env::args()
            .skip(1)
            .filter_map(
                |path| {
                    println!("{:-^30}", path);
                    match std::fs::read_to_string(&path) {
                        Ok(string) => Some(string),
                        Err(_err) => {
                            println!("Failed to read `{}` to string. Check location, permissions, and encoding of the file.", path);
                            None
                        },
                    }
                }
            )
            .for_each(
                |yaml_parse_result| match crate::verify_yaml(&yaml_parse_result) {
                    Ok(yaml) => println!("{:#?}", yaml),
                    Err(err) => {
                        errors_encountered = true;
                        eprintln!("{}", err);
                    },
                }
            );
    }
}
