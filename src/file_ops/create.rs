use std::fs;
use std::fs::DirBuilder;

pub fn create_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create <file>");
        return;
    }

    match fs::write(args[1], "") {
        Ok(_) => println!("File created"),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn create_directory(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create_directory <folder path>");
        return;
    }
    match DirBuilder::new().recursive(true).create(args[1]) {
        Ok(_) => println!("Directory created"),
        Err(e) => println!("Error creating folder: {}", e),
    }
}
