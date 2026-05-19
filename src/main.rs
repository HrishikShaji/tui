mod download;
mod error;
mod file_ops;

use std::io::{self, Write};

fn main() {
    println!("Filemanager CLI");
    println!("Type 'help' for commands");
    println!("Type 'exit' to quit");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        io::stdin().read_line(&mut input).expect("Failed to read");

        let input = input.trim();

        if input == "exit" {
            break;
        }

        if input == "help" {
            println!("Commands:");
            println!("  create <file>");
            println!("  write <file> <content>");
            println!("  read <file>");
            println!("  delete <file>");
            println!("  file_type <file>");
            println!("  create_directory <directory>");
            println!("  read_entries <path>");
            println!("  compress_file <source> <target>");
            println!("  decompress_file <source>");
            println!("  full_path <path>");
            println!("  copy_file <source> <target");
            println!("  copy_and_replace_file <source> <target");
            continue;
        }

        let parts: Vec<&str> = input.splitn(3, ' ').collect();

        match parts[0] {
            "create" => file_ops::create::create_file(parts),
            "write" => file_ops::write::write_file(parts),
            "read" => file_ops::read::read_file(parts),
            "delete" => file_ops::delete::delete_file(parts),
            "file_type" => file_ops::read::get_file_type(parts),
            "create_directory" => file_ops::create::create_directory(parts),
            "read_entries" => file_ops::read::read_entries(parts),
            "compress_file" => file_ops::compress::compress_file(parts),
            "decompress_file" => file_ops::compress::decompress_file(parts),
            "full_path" => file_ops::read::full_path(parts),
            "copy_file" => file_ops::create::copy_file(parts),
            "copy_and_replace_file" => file_ops::create::copy_and_replace_file(parts),

            _ => {
                println!("Unknown command");
            }
        }
    }

    println!("Goodbye!");
}
