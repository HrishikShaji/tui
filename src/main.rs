mod commands;
mod download;
mod error;
mod file_ops;
mod http;
mod utils;

use std::io::{self, Write};

struct Command {
    name: String,
    usage: String,
}

#[tokio::main]
async fn main() {
    println!("Filemanager CLI");
    println!("Type 'help' for commands");
    println!("Type 'exit' to quit");

    let commands: Vec<Command> = vec![
        Command {
            name: String::from("create"),
            usage: String::from("create <file>"),
        },
        Command {
            name: String::from("write"),
            usage: String::from("write <file> <content>"),
        },
        Command {
            name: String::from("read"),
            usage: String::from("read <file>"),
        },
        Command {
            name: String::from("delete"),
            usage: String::from("delete <file>"),
        },
        Command {
            name: String::from("file_type"),
            usage: String::from("file_type <file>"),
        },
        Command {
            name: String::from("create_directory"),
            usage: String::from("create_directory <directory>"),
        },
        Command {
            name: String::from("read_entries"),
            usage: String::from("read_entries <path>"),
        },
        Command {
            name: String::from("compress_file"),
            usage: String::from("compress_file <source> <target>"),
        },
        Command {
            name: String::from("decompress_file"),
            usage: String::from("decompress_file <source>"),
        },
        Command {
            name: String::from("full_path"),
            usage: String::from("full_path <path>"),
        },
        Command {
            name: String::from("copy_file"),
            usage: String::from("copy_file <source> <target"),
        },
        Command {
            name: String::from("copy_and_replace_file"),
            usage: String::from("copy_and_replace_file <source> <target"),
        },
        Command {
            name: String::from("get"),
            usage: String::from("get <url>"),
        },
        Command {
            name: String::from("ip_address"),
            usage: String::from("ip_address <version>"),
        },
    ];

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
            for command in &commands {
                println!("{}", command.name);
            }
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
            "get" => http::get::get(parts).await,
            "ip_address" => commands::system::get_ip_address(parts),

            _ => {
                println!("Unknown command");
            }
        }
    }

    println!("Goodbye!");
}
