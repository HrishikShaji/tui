use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::ToolDefinition;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, BufRead, Write};

use rig::client::CompletionClient;

use crate::llm::rig::{create_ollama_client, create_voice_agent};

// ─── Configuration ───────────────────────────────────────────────────

pub struct RigGenerateConfig {
    pub base_url: String,
    pub model: String,
    pub preamble: String,
}

impl Default for RigGenerateConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            preamble: "You are a helpful assistant running locally via Ollama.".to_string(),
        }
    }
}

// ─── Adder tool ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AddArgs {
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = std::convert::Infallible;
    type Args = AddArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "First number" },
                    "y": { "type": "number", "description": "Second number" }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.x + args.y)
    }
}

// ─── Service entry point ─────────────────────────────────────────────

/// Interactive async REPL loop using Ollama via rig.
pub async fn generate(config: RigGenerateConfig) {
    println!();
    println!("=== Rig Agent (Ollama) ===");
    println!("Endpoint : {}", config.base_url);
    println!("Model    : {}", config.model);
    println!("Preamble : {}", config.preamble);
    println!();
    println!("Type your message, or 'exit' to quit.");
    println!();

    let client = match create_ollama_client(&config.base_url) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to Ollama: {}", e);
            return;
        }
    };

    let agent = create_voice_agent(&client, &config.model, &config.preamble);

    let stdin = io::stdin();

    loop {
        print!("rig> ");
        io::stdout().flush().unwrap();

        let mut user_input = String::new();
        match stdin.lock().read_line(&mut user_input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        let user_input = user_input.trim();
        if user_input.is_empty() {
            continue;
        }
        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            println!("Leaving Rig agent.");
            break;
        }

        print!("Assistant: ");
        io::stdout().flush().unwrap();

        let mut stream = agent.stream_prompt(user_input).await;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    print!("{}", t.text);
                    io::stdout().flush().unwrap();
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("\nStreaming error: {}", e);
                    break;
                }
            }
        }

        println!();
    }
}

// ─── Single-shot call (used by registry `ai` command) ────────────────

/// Handle a single prompt from the command line (e.g. `ai <query>`).
pub async fn call_agent(args: Vec<String>) {
    let prompt = &args[1];

    let config = RigGenerateConfig::default();
    let client = match create_ollama_client(&config.base_url) {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let agent = client
        .agent(&config.model)
        .preamble(
            "You are a helpful assistant running locally via Ollama that does basic calculations.",
        )
        .tool(Adder)
        .build();

    let mut stream = agent.stream_prompt(prompt).await;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                print!("{}", t.text);
                io::stdout().flush().unwrap();
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Streaming error: {}", e);
                break;
            }
        }
    }

    println!();
}
