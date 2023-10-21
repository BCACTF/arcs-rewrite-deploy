static YAML: &str = r###"
categories:
    - webex
value: 50
flag: aaaaaaaaa
description: |-
    Describing this is so fun and I love writing words.
    Y'know?
hints:
    - How do forms send values?
authors:
    - Bloop
visible: true
name: Hidden Values
"###;

fn main() {
    use arcs_yaml_editor::*;

    let mut yaml = YAML.to_string();

    println!("-------");
    println!("{yaml}");

    yaml = try_replace_name(&yaml, "a cool\nnew name!").unwrap();
    println!("-------");
    println!("{yaml}");

    yaml = try_replace_points(&yaml, 25).unwrap();
    println!("-------");
    println!("{yaml}");

    yaml = try_replace_description(&yaml, "I don't like describing things").unwrap();
    println!("-------");
    println!("{yaml}");

    yaml = try_replace_tags(&yaml, &["tag1", "tag2", "tag3"]).unwrap();
    println!("-------");
    println!("{yaml}");

    yaml = try_replace_tags(&yaml, &[]).unwrap();
    println!("-------");
    println!("{yaml}");

    println!("-------");
    // let yaml_shape = arcs_yaml_editor::EditableYaml::try_new(YAML.to_string()).unwrap();


    // println!("{yaml_shape:#?}");

    // let marked_mapping = yaml_shape.get_marked().as_mapping().unwrap();

    // let idx_of_name = marked_mapping.iter().enumerate()
    //     .find_map(|(idx, (key, _))| (key.as_str() == "name").then_some(idx))
    //     .unwrap();

    // let (value_start_idx, value_end_idx) = yaml_shape.get_span_of_entry("description").unwrap();
    // println!("||{}||", &YAML[value_start_idx..value_end_idx]);

    // let locations = arcs_yaml_editor::yaml_layout_info::Locations::try_find(YAML).unwrap();

    // println!("{locations:#?}");
    // println!(
    //     "---\n{}\n---\n{}\n---\n{}\n---\n{}\n---\n{:?}",
    //     &YAML[locations.points.0..locations.points.1],
    //     &YAML[locations.name.0..locations.name.1],
    //     &YAML[locations.description.0..locations.description.1],
    //     &YAML[locations.categories.0..locations.categories.1],
    //     locations.tags.map(|(s, e)| &YAML[s..e])
    // );
    // println!("{:#?}");
    // println!("{locations:#?}");
    // println!("{locations:#?}");

    // println!("-------");
    // let name = yaml_shape.get_marked().as_mapping().unwrap().get_node("name").unwrap();
    // let name_span = yaml_shape.get_marked().as_mapping().unwrap();
    // let line = name.span().start().unwrap().line();
    // let col = name.span().start().unwrap().column();
    // let name_in_yaml: String = YAML.split("\n").nth(line - 1).unwrap().chars().skip(col - 1).collect();
    // println!("{name_span:#?}");
    // println!("{name:#?}");
    // println!("{name_in_yaml:#?}");
    // println!("-------");
}
