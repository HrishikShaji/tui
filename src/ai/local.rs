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
pub async fn call_agent(args: Vec<&str>) {
    let prompt = args[1];
    let model = "llama3.2";
    let base_url = "http://localhost:11434";

    let client = match Client::<ollama::OllamaExt>::builder()
        .api_key(Nothing)
        .base_url(base_url)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };
    let agent = client
        .agent(model)
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
            Ok(_) => {} // ignore ToolCall, Final, multi-turn items, etc.
            Err(e) => {
                eprintln!("Streaming error: {}", e);
                break;
            }
        }
    }

    println!("");
}
