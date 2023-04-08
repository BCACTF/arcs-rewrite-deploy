use arcs_yaml_parser::correctness::{FlagCorrectness, CategoryCorrectness, YamlCorrectness, PointCorrectness};

use std::borrow::Cow;

const CATEGORIES_STR: &str = "misc,binex,foren,crypto,webex,rev";
const COMPETITION_NAME: &str = "bcactf";
const POINT_MULT: u64 = 25;

pub fn main() {
    let categories = std::env::var("CATEGORIES").ok();
    let comp_name = std::env::var("COMPNAME").ok();
    let point_multiple: Option<u64> = std::env::var("POINT_MULT")
        .as_ref()
        .map(String::as_str).map(str::parse)
        .map(Result::ok).ok().flatten();

    let category_correctness = if let Some(category_names) = categories {
        let cats = if &category_names == "DEFAULT" {
            CATEGORIES_STR
        } else {
            &category_names
        };

        let names: Vec<_> = cats.split(',').map(str::to_string).map(Cow::Owned).collect();
        CategoryCorrectness::List { names: names.into(), requires_case_match: false }
    } else {
        CategoryCorrectness::AnyStr
    };
    let flag_correctness = if let Some(comp_name) = comp_name {
        let comp_name = if &comp_name == "DEFAULT" {
            COMPETITION_NAME.into()
        } else {
            comp_name.into()
        };
        FlagCorrectness::CompName(comp_name)
    } else {
        FlagCorrectness::None
    };
    let point_correctness = if let Some(points) = point_multiple {
        let points = if points == 0 {
            POINT_MULT
        } else {
            points
        };

        PointCorrectness::Multiple(points)
    } else {
        PointCorrectness::None
    };


    let yaml_correctness = YamlCorrectness::default()
        .with_flag(flag_correctness)
        .with_cats(category_correctness)
        .with_pnts(point_correctness);

    arcs_yaml_parser::__main::main(yaml_correctness);
}
