use std::{fs, path::Path};

pub fn write_file(args: Vec<String>) {
    if args.len() < 3 {
        println!("Usage: write <file> <content>");
        return;
    }

    let file_path = Path::new(&args[1]);

    if !file_path.exists() {
        println!("The file does not exist");
        return;
    }

    match fs::write(&args[1], &args[2]) {
        Ok(_) => println!("Written successfully"),
        Err(e) => println!("Error: {}", e),
    }
}
