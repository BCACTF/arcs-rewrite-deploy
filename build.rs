fn get_file(url_env: &str, path_env: &str) -> Result<String, String> {
    match (std::env::var(url_env),  std::env::var(path_env)) {
        (Ok(_), Ok(_)) => Err(format!("Both {url_env} and {path_env} are set. Please only set one of them")),
        (Ok(url), Err(_)) => {
            let result = reqwest::blocking::get(url).map_err(|e| format!("Failed to get request schema file: {}", e))?;
            let text = result.text().map_err(|e| format!("Failed to get text from schema file request: {}", e))?;
            Ok(text)
        },
        (Err(_), Ok(path)) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("Failed to read schema file: {}", e))?;
            Ok(text)
        },
        (Err(_), Err(_)) => Err(format!("Neither {url_env} nor {path_env} are set. Please set one of them.")),
    }
}

fn text_to_schema(text: &str) -> Result<typify::TypeSpace, String> {
    use serde_json::from_str;
    use schemars::schema::RootSchema;
    use typify::{TypeSpace, TypeSpaceSettings };

    let schema = from_str::<RootSchema>(text).map_err(|e| format!("Failed to parse schema file: {}", e))?;
    let mut type_space = TypeSpace::new(TypeSpaceSettings::default().with_struct_builder(false));
    type_space.add_root_schema(schema).map_err(|e| format!("Failed to add schema to type space: {}", e))?;

    Ok(type_space)
}

fn schema_to_file_output(schema: typify::TypeSpace) -> Result<String, String> {
    use syn::{ parse2, File };
    use prettyplease::unparse;

    let mut file_contents = "//! This file is autogenerated by the build script.\n//! Please do not modify it.".to_string();

    file_contents.push('\n');
    file_contents.push_str("use serde::{Deserialize, Serialize};");
    file_contents.push('\n');
    file_contents.push('\n');

    let parsed = parse2::<File>(schema.to_stream())
        .map_err(|e| format!("Failed to parse generated code: {e}"))?;
    let pretty_printed = unparse(&parsed);

    file_contents.push_str(&pretty_printed);

    Ok(file_contents)
}




fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    let incoming_text = get_file("API_INCOMING_SCHEMA_URL", "API_INCOMING_SCHEMA_FILE_PATH").unwrap();
    let outgoing_text = get_file("API_OUTGOING_SCHEMA_URL", "API_OUTGOING_SCHEMA_FILE_PATH").unwrap();

    let incoming_typespace = text_to_schema(&incoming_text).unwrap();
    let outgoing_typespace = text_to_schema(&outgoing_text).unwrap();

    let incoming_file_contents = schema_to_file_output(incoming_typespace).unwrap();
    let outgoing_file_contents = schema_to_file_output(outgoing_typespace).unwrap();

    std::fs::create_dir_all("./src/server/utils/api_types").expect("Failed to create api_types directory");

    std::fs::write(
        "./src/server/utils/api_types/incoming.rs",
        incoming_file_contents,
    ).expect("Failed to write incoming typedefs");
    std::fs::write(
        "./src/server/utils/api_types/outgoing.rs",
        outgoing_file_contents,
    ).expect("Failed to write outgoing typedefs");


}
