use anyhow::Result;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{Client, CompletionClient, Nothing};
use rig::completion::{Prompt, ToolDefinition};
use rig::providers::ollama;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, Write};

// ─── Adder tool (used by the `ai` demo command) ─────────────────────

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

// ─── Builder functions ───────────────────────────────────────────────

/// Create an Ollama client pointing at `http://localhost:11434`.
pub fn create_ollama_client() -> Result<Client<ollama::OllamaExt>> {
    let client = Client::<ollama::OllamaExt>::builder()
        .api_key(Nothing)
        .base_url("http://localhost:11434")
        .build()?;
    Ok(client)
}

/// Build a streaming voice-assistant agent on top of the given Ollama client.
pub fn create_voice_agent(
    client: &Client<ollama::OllamaExt>,
) -> rig::agent::Agent<ollama::CompletionModel> {
    client
        .agent("llama3.2")
        .preamble(
            "You are a helpful voice assistant. \
             Keep responses concise and conversational.",
        )
        .build()
}

// ─── Standalone demo (called by the `ai` REPL command) ──────────────

pub async fn call_agent(args: Vec<String>) {
    let prompt = &args[1];

    let client = match create_ollama_client() {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let agent = client
        .agent("llama3.2")
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

    println!("");
}
