use std::fs;

pub fn write_file(args: Vec<&str>) {
    if args.len() < 3 {
        println!("Usage: write <file> <content>");
        return;
    }

    match fs::write(args[1], args[2]) {
        Ok(_) => println!("Written successfully"),
        Err(e) => println!("Error: {}", e),
    }
}
