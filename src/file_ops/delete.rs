use std::fs;

pub fn delete_file(args: Vec<&str>) {
    if args.len() < 2 {
        println!("Usage: delete <file>");
        return;
    }

    match fs::remove_file(args[1]) {
        Ok(_) => println!("Deleted"),
        Err(e) => println!("Error: {}", e),
    }
}
