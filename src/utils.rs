use std::io::{self, Write};

pub fn check_confirmation() -> bool {
    print!(">> yes or no ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read");

    input.trim() == "yes"
}

pub fn model_path(file: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join(file)
        .to_string_lossy()
        .to_string()
}

pub fn models_dir() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .to_string_lossy()
        .to_string()
}
