mod ai;
mod commands;
mod download;
mod error;
mod file_ops;
mod http;
mod registry;
mod utils;
mod voice;

use registry::CommandHandler;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    println!("Filemanager CLI");
    println!("Type 'help' for commands");
    println!("Type 'exit' to quit");

    let registry = registry::build_registry();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        io::stdin().read_line(&mut input).expect("Failed to read");

        let input = input.trim();

        if input == "exit" {
            break;
        }

        if input == "voice" {
            voice::local::record_and_play();
        }

        if input == "help" {
            println!("Commands:");

            for (name, command) in &registry {
                println!("{name} -> {}", command.usage);
            }

            continue;
        }

        let parts: Vec<String> = input.splitn(3, ' ').map(String::from).collect();

        if parts.is_empty() {
            continue;
        }

        let command_name = &parts[0];

        match registry.get(command_name.as_str()) {
            Some(command) => match &command.handler {
                CommandHandler::Sync(handler) => {
                    handler(parts);
                }

                CommandHandler::Async(handler) => {
                    handler(parts).await;
                }
            },

            None => {
                println!("Unknown command");
            }
        }
    }

    println!("Goodbye!");
}
