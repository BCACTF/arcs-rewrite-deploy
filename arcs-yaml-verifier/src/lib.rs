mod categories;
mod lists;
mod flag;
mod files;
mod deploy;

mod structs;
mod accessors;

mod correctness;


// Serde
use serde_yaml::{
    Mapping as YamlMapping,
    Value as YamlValue,
};


// Parsing functions, types, and errors
use {
    flag::get_flag,
    files::file_list,
    lists::as_str_list,
};

use {
    files::Files,
    flag::Flag,
    lists::structs::{ Authors, Hints },
    categories::Categories,
};
pub use deploy::structs::DeployOptions;

use {
    lists::structs::{ AuthorError, HintError },
    flag::FlagError,
    categories::CategoryError,
};


// Yaml ValueType stuff
use structs::{
    ValueType,
    get_type,
};


// Verification stuff
pub use structs::{
    YamlVerifyError,
    YamlAttribVerifyError
};
pub use correctness::YamlCorrectness;


// misc
use crate::{files::Flop, deploy::parse_deploy};





#[derive(PartialEq, Debug)]
pub struct YamlShape {
    authors: Authors,
    categories: Categories,
    hints: Hints,
    files: Option<Files>,

    deploy: Option<DeployOptions>,

    points: u64,
    flag: Flag,
    
    name: String,
    description: String,

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


fn verify_yaml(yaml_text: &str, correctness_options: Option<YamlCorrectness>) -> Result<YamlShape, YamlVerifyError> {
    use YamlVerifyError::*;
    use YamlAttribVerifyError::*;

    let correctness = correctness_options.unwrap_or_default();

    let base: YamlValue = serde_yaml::from_str(yaml_text).map_err(Unparsable)?;
    let base: &YamlMapping = if let Some(base) = base.as_mapping() { base } else {
        return Err(BaseNotMap(get_type(&base)))
    };

    let (
        categories,
        authors,
        hints,
        files,
    ) = {
        let categories = base
            .get("categories")
            .map_or(Err(CategoryError::MissingKey), categories::value_to_categories)
            .map_err(Categories);
    
        let authors = base
            .get("authors")
            .map_or(Err(AuthorError::MissingKey), as_str_list)
            .map_err(Authors);
    
        let hints = base
            .get("hints")
            .map_or(Err(HintError::MissingKey), as_str_list)
            .map_err(Hints);

        let files = base
            .get("files")
            .map(file_list)
            .flop()
            .map_err(Files);
        
        (categories, authors, hints, files)
    };


    let deploy = base
        .get("deploy")
        .map(parse_deploy)
        .flop()
        .map_err(Deploy);


    let points = if let Some(point_val) = base.get("value") {
        point_val.as_u64().ok_or_else(|| PointsNotInt(get_type(point_val)))
    } else { Err(PointsNotInt(ValueType::NULL)) };

    let flag = base
        .get("flag")
        .map_or(Err(FlagError::MissingKey), get_flag)
        .map_err(Flag);


    let name = if let Some(name_val) = base.get("name") {
        name_val.as_str().map(str::to_string).ok_or_else(|| NameNotString(get_type(name_val)))
    } else { Err(NameNotString(ValueType::NULL)) };

    let description = if let Some(desc_val) = base.get("description") {
        desc_val.as_str().map(str::to_string).ok_or_else(|| DescNotString(get_type(desc_val)))
    } else {  Err(DescNotString(ValueType::NULL)) };


    let visible = if let Some(point_val) = base.get("visible") {
        point_val.as_bool().ok_or_else(|| VisNotBool(get_type(point_val)))
    } else { Err(VisNotBool(ValueType::NULL)) };

    let (
        authors,
        categories,
        hints,
        files,
        
        deploy,

        points,
        flag,
        
        name,
        description,

        visible,
    ) = collect_errors!(
        authors,
        categories,
        hints,
        files,
        
        deploy,

        points,
        flag,

        name,
        description,
        
        visible,
    ).map_err(PartErrors)?;

    let shape = YamlShape {
        authors, categories, hints, files,
        deploy,
        points, flag,
        name, description,
        visible,
    };
    correctness.verify(&shape).map_err(Correctness)?;

    Ok(shape)
}

#[doc(hidden)]
pub mod __main {
    use std::borrow::Cow::{ self, Borrowed };

    use crate::correctness::*;

    const CATEGORIES: &[Cow<'static, str>] = &[
        Borrowed("misc"),
        Borrowed("binex"),
        Borrowed("foren"),
        Borrowed("crypto"),
        Borrowed("webex"),
    ];

    pub fn main() {
        let mut errors_encountered = false;

        let yaml_correctness = YamlCorrectness::default()
            .with_flag(FlagCorrectness::CompName("bcactf".into()))
            .with_cats(CategoryCorrectness::List {
                names: Borrowed(CATEGORIES),
                requires_case_match: false,
            })
            .with_pnts(PointCorrectness::Multiple(25));

        std::env::args()
            .skip(1)
            .filter_map(
                |path| {
                    println!("{path:-^30}");
                    match std::fs::read_to_string(&path) {
                        Ok(string) => Some(string),
                        Err(_err) => {
                            println!("Failed to read `{path}` to string. Check location, permissions, and encoding of the file.");
                            None
                        },
                    }
                }
            )
            .for_each(
                |yaml_parse_result| match crate::verify_yaml(&yaml_parse_result, Some(yaml_correctness.clone())) {
                    Ok(yaml) => println!("{yaml:#?}"),
                    Err(err) => {
                        errors_encountered = true;
                        eprintln!("{err}");
                    },
                }
            );
    }
}
