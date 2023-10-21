use hashlink::LinkedHashMap;
use yaml_rust::{ Yaml, YamlEmitter };
use crate::locations::Locations;

fn get_hash(key: &str, value: Yaml) -> Yaml {
    let hash_map: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::from_iter(
        std::iter::once(
            (
                Yaml::String(key.to_string()),
                value,
            )
        )
    );

    Yaml::Hash(hash_map)
}

fn format_yaml_pretty(yaml: &Yaml) -> Option<String> {
    let mut replacement = String::new();
    let mut emitter = YamlEmitter::new(&mut replacement);
    emitter.compact(false);
    emitter.multiline_strings(true);
    emitter.dump(yaml).ok()?;

    Some(replacement.trim_start_matches("---\n").to_string())
}

pub fn try_replace(yaml_str: &str, key: &str, new_val: Yaml, span: (usize, usize)) -> Option<String> {
    let yaml = get_hash(key, new_val);

    let before: &str = &yaml_str[..span.0];
    let after: &str = &yaml_str[span.1..];

    Some(format!("{}{}{}", before, format_yaml_pretty(&yaml)?, after))
}

pub fn try_replace_name(yaml_str: &str, new_name: &str) -> Option<String> {
    let locations = Locations::try_find(yaml_str)?;

    try_replace(yaml_str, "name", Yaml::String(new_name.to_string()), locations.name)
}

pub fn try_replace_points(yaml_str: &str, new_points: u64) -> Option<String> {
    let locations = Locations::try_find(yaml_str)?;

    try_replace(yaml_str, "value", Yaml::Integer(new_points as i64), locations.points)
}

pub fn try_replace_description(yaml_str: &str, new_description: &str) -> Option<String> {
    let locations = Locations::try_find(yaml_str)?;

    try_replace(yaml_str, "description", Yaml::String(new_description.to_string()), locations.description)
}

pub fn try_replace_categories<T: ToOwned<Owned = String>>(yaml_str: &str, new_categories: &[T]) -> Option<String> {
    let locations = Locations::try_find(yaml_str)?;

    let new_categories = new_categories.iter().map(|s| Yaml::String(s.to_owned())).collect::<Vec<_>>();

    try_replace(yaml_str, "categories", Yaml::Array(new_categories), locations.categories)
}

pub fn try_replace_tags<T: ToOwned<Owned = String>>(yaml_str: &str, new_tags: &[T]) -> Option<String> {
    let locations = Locations::try_find(yaml_str)?;

    if new_tags.len() == 0 {
        return if let Some((s, e)) = locations.tags {
            Some(format!("{}{}", &yaml_str[..s], &yaml_str[e..]))
        } else {
            Some(yaml_str.to_string())
        };
    }

    let new_tags = new_tags.iter().map(|s| Yaml::String(s.to_owned())).collect::<Vec<_>>();

    if let Some(span) = locations.tags {
        try_replace(yaml_str, "tags", Yaml::Array(new_tags), span)
    } else {
        let yaml = get_hash("tags", Yaml::Array(new_tags));
        Some(format!(
            "{}\n{}",
            yaml_str.trim_end_matches('\n'),
            format_yaml_pretty(&yaml)?,
        ))
    }
}




