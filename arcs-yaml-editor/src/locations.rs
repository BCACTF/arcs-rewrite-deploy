#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Locations {
    pub points: (usize, usize),
    pub name: (usize, usize),
    pub description: (usize, usize),
    pub categories: (usize, usize),
    pub tags: Option<(usize, usize)>,
    pub visible: (usize, usize),
}

impl Locations {
    pub fn try_find(yaml_str: &str) -> Option<Locations> {
        let yaml = marked_yaml::parse_yaml(0, yaml_str).ok()?;

        let points = Self::get_span_of_entry(&yaml, yaml_str, "value")?;
        let name = Self::get_span_of_entry(&yaml, yaml_str, "name")?;
        let description = Self::get_span_of_entry(&yaml, yaml_str, "description")?;
        let categories = Self::get_span_of_entry(&yaml, yaml_str, "categories")?;
        let tags = Self::get_span_of_entry(&yaml, yaml_str, "tags");
        let visible = Self::get_span_of_entry(&yaml, yaml_str, "visible")?;

        Some(Locations {
            points,
            name,
            description,
            categories,
            tags,
            visible,
        })
    }

    fn get_span_of_entry(marked_yaml: &marked_yaml::Node, yaml_text: &str, key_name: &str) -> Option<(usize, usize)> {
        let marked_mapping = marked_yaml.as_mapping()?;

        let (idx_of_entry, value_start) = marked_mapping
            .iter().enumerate()
            .find_map(
                |(idx, (key, _))| {
                    if key.as_str() == key_name {
                        Some((idx, key.span().start()?))
                    } else {
                        None
                    }
                }
            )?;

        // let value_start = marked_mapping.get_node(key_name)?.span().start()?;
        let value_end = marked_mapping
            .iter().enumerate()
            .find_map(|(idx, (key, _))| (idx == idx_of_entry+1).then_some(key.span()))
            .and_then(|span| span.start());

        let value_start_idx = yaml_text.split("\n").take(value_start.line() - 1).fold(0, |acc, line| acc + line.len() + 1) + value_start.column() - 1;
        let value_end_idx = if let Some(value_end) = value_end {
            let col_idx = yaml_text
                .split("\n")
                .take(value_end.line() - 1)
                .fold(0, |acc, line| acc + line.len() + 1);
            let row_idx = value_end.column() - 1;

            col_idx + row_idx
        } else {
            yaml_text.len()
        };

        let value_end_idx = value_start_idx + yaml_text[value_start_idx..value_end_idx].trim_end().len();

        Some((value_start_idx, value_end_idx))
    }
}
