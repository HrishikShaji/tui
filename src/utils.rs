use std::io::{self, Write};
use std::path::PathBuf;

pub fn check_confirmation() -> bool {
    print!(">> yes or no ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read");

    input.trim() == "yes"
}

pub fn models_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        // Development:
        // project/models
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models")
    }

    #[cfg(not(debug_assertions))]
    {
        // Release:
        // beside executable
        let exe = std::env::current_exe().expect("failed to get executable path");

        exe.parent()
            .expect("failed to get executable directory")
            .join("models")
    }
}

pub fn model_path(file: &str) -> String {
    models_dir().join(file).to_string_lossy().to_string()
}
