use std::fs;
use std::fs::DirBuilder;
use std::path::Path;

pub fn copy_file(args: Vec<&str>) {
    if args.len() < 3 {
        println!("Usage: copy <source> <target>");
        return;
    }

    let source_file_path = Path::new(args[1]);
    let target_file_path = Path::new(args[2]);

    if !source_file_path.exists() {
        println!("The source file does not exists");
        return;
    }

    if target_file_path.exists() {
        println!("The Target file path already exists try copy_and_replace_file");
        return;
    }

    match fs::copy(args[1], args[2]) {
        Ok(_) => println!("File copied successfully"),
        Err(e) => println!("Error copying file:{}", e),
    }
}

pub fn copy_and_replace_file(args: Vec<&str>) {
    if args.len() < 3 {
        println!("Usage: copy <source> <target>");
        return;
    }

    let source_file_path = Path::new(args[1]);

    if !source_file_path.exists() {
        println!("The source file does not exists");
        return;
    }

    match fs::copy(args[1], args[2]) {
        Ok(_) => println!("File copied successfully"),
        Err(e) => println!("Error copying file:{}", e),
    }
}

pub fn create_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: create <file>");
        return;
    }

    let file_path = Path::new(args[1]);

    if file_path.exists() {
        println!("The file already exists");
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

    let file_path = Path::new(args[1]);

    if file_path.exists() {
        println!("The directory already exists");
        return;
    }

    match DirBuilder::new().recursive(true).create(args[1]) {
        Ok(_) => println!("Directory created"),
        Err(e) => println!("Error creating folder: {}", e),
    }
}
