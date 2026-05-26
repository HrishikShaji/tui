use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{Client, CompletionClient, Nothing};
use rig::completion::Prompt;
use rig::providers::ollama;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};

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
        .preamble("You are a helpful assistant running locally via Ollama.")
        .build();

    let mut stream = agent.stream_prompt(prompt).await;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                print!("{}", t.text);
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
