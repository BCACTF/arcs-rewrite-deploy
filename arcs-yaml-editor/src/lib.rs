mod locations;
mod replace;

pub use replace::*;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Modifications {
    name: Option<String>,
    desc: Option<String>,
    points: Option<u64>,
    categories: Option<Vec<String>>,
    tags: Option<Option<Vec<String>>>,
    visible: Option<bool>,
}

impl Modifications {
    pub fn apply(&self, yaml: &str) -> Option<String> {
        let mut yaml = yaml.to_string();

        if let Some(name) = &self.name {
            yaml = try_replace_name(&yaml, name)?;
        }

        if let Some(desc) = &self.desc {
            yaml = try_replace_description(&yaml, desc)?;
        }

        if let Some(points) = &self.points {
            yaml = try_replace_points(&yaml, *points)?;
        }

        if let Some(categories) = &self.categories {
            yaml = try_replace_categories(&yaml, categories)?;
        }

        if let Some(tags) = &self.tags {
            yaml = try_replace_tags(&yaml, tags.as_ref().map(Vec::as_slice).unwrap_or(&[]))?;
        }

        if let Some(visibility) = &self.visible {
            yaml = try_replace_visible(&yaml, *visibility)?;
        }

        Some(yaml)
    }
}
