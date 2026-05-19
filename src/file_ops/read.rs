use std::fs;
use std::path::Path;

pub fn read_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: read <file>");
        return;
    }

    let file_path = Path::new(args[1]);

    if !file_path.exists() {
        println!("The file does not exist");
        return;
    }

    match fs::read_to_string(args[1]) {
        Ok(content) => println!("{}", content),
        Err(e) => println!("Error: {}", e),
    }
}

pub fn get_file_type(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: file_type <file>");
        return;
    }

    let file_path = Path::new(args[1]);

    if !file_path.exists() {
        println!("The file does not exist");
        return;
    }

    match fs::metadata(args[1]) {
        Ok(metadata) => {
            println!("{:?}", metadata.file_type());
        }
        Err(e) => println!("Error figuring out the file type: {}", e),
    }
}

pub fn read_entries(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: read_entries <folder path>");
        return;
    }

    let file_path = Path::new(args[1]);

    if !file_path.exists() {
        println!("The directory does not exist");
        return;
    }

    let entries = match fs::read_dir(args[1]) {
        Ok(dir) => dir,
        Err(e) => {
            println!("Error reading dir: {}", e);
            return;
        }
    };

    for entry in entries {
        match entry {
            Ok(file) => {
                println!("{:?}", file.path());
            }
            Err(e) => {
                println!("Error reading file: {}", e);
            }
        }
    }
}
