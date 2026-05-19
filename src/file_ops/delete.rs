use std::{fs, path::Path};

pub fn delete_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: delete <file>");
        return;
    }

    let file_path = Path::new(args[1]);

    if !file_path.exists() {
        println!("The file does not exist");
        return;
    }

    match fs::remove_file(args[1]) {
        Ok(_) => println!("Deleted"),
        Err(e) => println!("Error: {}", e),
    }
}
