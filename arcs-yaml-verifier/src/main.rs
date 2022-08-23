pub fn main() {
    let mut errors_encountered = false;

    std::env::args()
        .skip(1)
        .filter_map(
            |path| {
                println!("{:-^30}", path);
                match std::fs::read_to_string(&path) {
                    Ok(string) => Some(string),
                    Err(err) => {
                        println!("Failed to read `{}` to string. Check location, permissions, and encoding of the file.", path);
                        None
                    },
                }
            }
        )
        .for_each(
            |yaml_parse_result| match arcs_yaml_verifier::verify_yaml(&yaml_parse_result) {
                Ok(yaml) => println!("{}", yaml),
                Err(err) => {
                    errors_encountered = true;
                    eprintln!("{}", err);
                },
            }
        );

}
