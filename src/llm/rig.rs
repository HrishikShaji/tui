use anyhow::Result;
use rig::client::{Client, CompletionClient, Nothing};
use rig::providers::ollama;

// ─── Builder functions ───────────────────────────────────────────────

/// Create an Ollama client pointing at the given base URL.
pub fn create_ollama_client(base_url: &str) -> Result<Client<ollama::OllamaExt>> {
    let client = Client::<ollama::OllamaExt>::builder()
        .api_key(Nothing)
        .base_url(base_url)
        .build()?;
    Ok(client)
}

/// Build a streaming voice-assistant agent on top of the given Ollama client.
pub fn create_voice_agent(
    client: &Client<ollama::OllamaExt>,
    model: &str,
    preamble: &str,
) -> rig::agent::Agent<ollama::CompletionModel> {
    client
        .agent(model)
        .preamble(preamble)
        .build()
}


