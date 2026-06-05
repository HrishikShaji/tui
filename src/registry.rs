use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{llm, tools, voice};

pub type SyncHandler = fn(Vec<String>);

pub type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub type AsyncHandler = Box<dyn Fn(Vec<String>) -> BoxFuture + Send + Sync>;

pub enum CommandHandler {
    Sync(SyncHandler),
    Async(AsyncHandler),
}

pub struct Command {
    pub usage: &'static str,
    pub handler: CommandHandler,
}

pub fn async_handler<F, Fut>(f: F) -> impl Fn(Vec<String>) -> BoxFuture
where
    F: Fn(Vec<String>) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    move |args| Box::pin(f(args))
}

pub fn build_registry() -> HashMap<&'static str, Command> {
    let mut registry: HashMap<&str, Command> = HashMap::new();

    registry.insert(
        "create",
        Command {
            usage: "create <file>",
            handler: CommandHandler::Sync(tools::file_ops::create::create_file),
        },
    );

    registry.insert(
        "write",
        Command {
            usage: "write <file> <content>",
            handler: CommandHandler::Sync(tools::file_ops::write::write_file),
        },
    );

    registry.insert(
        "read",
        Command {
            usage: "read <file>",
            handler: CommandHandler::Sync(tools::file_ops::read::read_file),
        },
    );

    registry.insert(
        "delete",
        Command {
            usage: "delete <file>",
            handler: CommandHandler::Sync(tools::file_ops::delete::delete_file),
        },
    );

    registry.insert(
        "file_type",
        Command {
            usage: "file_type <file>",
            handler: CommandHandler::Sync(tools::file_ops::read::get_file_type),
        },
    );

    registry.insert(
        "create_directory",
        Command {
            usage: "create_directory <directory>",
            handler: CommandHandler::Sync(tools::file_ops::create::create_directory),
        },
    );

    registry.insert(
        "read_entries",
        Command {
            usage: "read_entries <path>",
            handler: CommandHandler::Sync(tools::file_ops::read::read_entries),
        },
    );

    registry.insert(
        "compress_file",
        Command {
            usage: "compress_file <source> <target>",
            handler: CommandHandler::Sync(tools::file_ops::compress::compress_file),
        },
    );

    registry.insert(
        "decompress_file",
        Command {
            usage: "decompress_file <source>",
            handler: CommandHandler::Sync(tools::file_ops::compress::decompress_file),
        },
    );

    registry.insert(
        "full_path",
        Command {
            usage: "full_path <path>",
            handler: CommandHandler::Sync(tools::file_ops::read::full_path),
        },
    );

    registry.insert(
        "copy_file",
        Command {
            usage: "copy_file <source> <target>",
            handler: CommandHandler::Sync(tools::file_ops::create::copy_file),
        },
    );

    registry.insert(
        "copy_and_replace_file",
        Command {
            usage: "copy_and_replace_file <source> <target>",
            handler: CommandHandler::Sync(tools::file_ops::create::copy_and_replace_file),
        },
    );

    registry.insert(
        "ip_address",
        Command {
            usage: "ip_address <version>",
            handler: CommandHandler::Sync(tools::system::system::get_ip_address),
        },
    );

    registry.insert(
        "ai",
        Command {
            usage: "ai <query>",
            handler: CommandHandler::Async(Box::new(async_handler(llm::rig::call_agent))),
        },
    );

    registry.insert(
        "get",
        Command {
            usage: "get <url>",
            handler: CommandHandler::Async(Box::new(async_handler(tools::network::get::get))),
        },
    );

    registry
}
