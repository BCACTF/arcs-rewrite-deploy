use std::borrow::Cow;
use std::fmt::{Debug, Display};
use regex::Regex;

use crate::YamlShape;
use crate::categories::structs::Category;


#[derive(Debug, Clone, PartialEq)]
pub struct YamlCorrectness {
    flag: FlagCorrectness,
    categories: CategoryCorrectness,
    points: PointCorrectness,
}

#[derive(Debug, Clone)]
pub enum FlagCorrectness {
    None,
    CompName(Cow<'static, str>),
    Regex(Regex),
}
impl PartialEq for FlagCorrectness {
    fn eq(&self, other: &Self) -> bool {
        use FlagCorrectness::{CompName, None, Regex};
        match (self, other) {
            (None, None) => true,
            (CompName(n1), CompName(n2)) => n1 == n2,
            (Regex(r1), Regex(r2)) => r1.as_str() == r2.as_str(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CategoryCorrectness {
    AnyStr,
    List {
        names: Cow<'static, [Cow<'static, str>]>,
        requires_case_match: bool,
    }
}

pub trait CanBePred: Fn(u64) -> bool + Debug {}

#[derive(Clone)]
pub enum PointCorrectness {
    None,
    Multiple(u64),
    Pred(std::rc::Rc<dyn CanBePred>),
}

impl PartialEq for PointCorrectness {
    fn eq(&self, other: &Self) -> bool {
        use PointCorrectness::{Multiple, None, Pred};
        match (self, other) {
            (None, None) => true,
            (Multiple(n1), Multiple(n2)) => n1 == n2,
            (Pred(p1), Pred(p2)) => std::ptr::eq(
                std::rc::Rc::as_ptr(p1) as *mut (),
                std::rc::Rc::as_ptr(p2) as *mut (),
            ),
            _ => false,
        }
    }
}
impl Debug for PointCorrectness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Multiple(n) => write!(f, "Multiple< of {n} >"),
            Self::Pred(_) => write!(f, "Predicate< unknown >"),
        }
    }
}

impl YamlCorrectness {
    pub fn check_flag(&self, flag: &str) -> bool { self.flag.check(flag) }
    pub fn check_cats<'a>(&self, categories: impl Iterator<Item = &'a str>) -> bool { self.categories.check(categories) }
    pub fn check_pnts(&self, points: u64) -> bool { self.points.check(points) }

    pub fn verify<'a>(&self, shape: &'a YamlShape) -> Result<&'a YamlShape, YamlCorrectness> {
        let flag_ok = if let crate::flag::Flag::String(flag) = &shape.flag {
            self.check_flag(flag)
        } else { true };
        let cats_ok = self.check_cats(shape.categories.iter().map(Category::as_str));
        let pnts_ok = self.check_pnts(shape.points);
        if flag_ok && cats_ok && pnts_ok {
            Ok(shape)
        } else {
            Err(Self {
                flag: if flag_ok { FlagCorrectness::None } else { self.flag.clone() },
                categories: if cats_ok { CategoryCorrectness::AnyStr } else { self.categories.clone() },
                points: if pnts_ok { PointCorrectness::None } else { self.points.clone() },
            })
        }
    }
}

impl FlagCorrectness {
    pub fn check(&self, flag: &str) -> bool {
        match self {
            Self::None => true,
            Self::CompName(name) => {
                if let Some(flag) = flag.strip_prefix(name.as_ref()) {
                    if let Some(flag) = flag.strip_prefix('{') {
                        flag.ends_with('}')
                    } else { false }
                } else { false }
            },
            Self::Regex(regex) => regex.is_match(flag),
        }
    }
}

impl CategoryCorrectness {
    pub fn check<'a>(&self, mut categories: impl Iterator<Item = &'a str>) -> bool {
        match self {
            Self::AnyStr => true,
            Self::List { names, requires_case_match } => {
                let case_match_predicate = |a: &str, b: &str| if *requires_case_match {
                    a == b
                } else {
                    a.to_lowercase() == b.to_lowercase()
                };
                
                let pred = |check| names
                    .iter()
                    .any(|valid| case_match_predicate(valid, check));

                categories.all(pred)
            },
        }
    }
}

impl PointCorrectness {
    pub fn check(&self, num: u64) -> bool {
        match self {
            Self::None => true,
            Self::Multiple(factor) => num % factor == 0,
            Self::Pred(pred) => pred(num),
        }
    }
}

impl Default for YamlCorrectness {
    fn default() -> Self {
        Self {
            flag: FlagCorrectness::None,
            categories: CategoryCorrectness::AnyStr,
            points: PointCorrectness::None,
        }
    }
}


impl YamlCorrectness {
    pub fn with_flag(self, flag: FlagCorrectness) -> Self { Self { flag, ..self } }
    pub fn with_cats(self, categories: CategoryCorrectness) -> Self { Self { categories, ..self } }
    pub fn with_pnts(self, points: PointCorrectness) -> Self { Self { points, ..self } }
}


impl YamlCorrectness {
    pub fn show_issue(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        if !matches!(self.flag, FlagCorrectness::None) {
            write!(f, "    ")?;
            self.flag.show_issue(f)?;
        }
        if !matches!(self.categories, CategoryCorrectness::AnyStr) {
            write!(f, "    ")?;
            self.categories.show_issue(f)?;
        }
        if !matches!(self.points, PointCorrectness::None) {
            write!(f, "    ")?;
            self.points.show_issue(f)?;
        }

        Ok(())
    }
}
impl FlagCorrectness {
    pub fn show_issue(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FlagCorrectness::*;
        match self {
            None => Ok(()),
            CompName(name) => writeln!(f, "The flag should be in the format of: `{name}{{<contents>}}`"),
            Regex(regex) => writeln!(f, "The flag must match the regex: `{}`", regex.as_str()),
        }
    }
}
impl CategoryCorrectness {
    pub fn show_issue(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CategoryCorrectness::*;
        match self {
            AnyStr => Ok(()),
            List {
                names,
                requires_case_match
            } => {
                write!(f, "The categories should be one of {names:?} (case ")?;
                if *requires_case_match {
                    writeln!(f, "SENSITIVE)")
                } else {
                    writeln!(f, "insensitive)")
                }
            },
        }
    }
}
impl PointCorrectness {
    pub fn show_issue(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PointCorrectness::*;
        match self {
            None => Ok(()),
            Multiple(n) => writeln!(f, "The point value MUST be multiple of {n}"),
            Pred(pred) => writeln!(f, "The flag must pass a predicate: `{pred:?}`"),
        }
    }
}


impl Display for YamlCorrectness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.show_issue(f)
    }
}
