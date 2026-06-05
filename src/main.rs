mod agents;
mod devices;
mod error;
mod llm;
mod registry;
mod stt;
mod tools;
mod tts;
mod utils;

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

        if input == "agent" {
            agents::non_streaming_agent::agent().await;
        }

        if input == "voice" {
            devices::mic_and_speaker::record_and_play();
        }

        if input == "llm" {
            agents::llama_generate::generate(agents::llama_generate::LlamaGenerateConfig::default());
        }

        if input == "stream" {
            agents::streaming_agent::agent().await;
        }

        if input == "tts" {
            agents::sherpa_speech_synthesis::speak(
                agents::sherpa_speech_synthesis::SpeechSynthesisConfig::default(),
            );
        }

        if input == "stt" {
            agents::sherpa_transcribe::transcribe(
                agents::sherpa_transcribe::TranscribeConfig::default(),
            );
        }

        if input == "rig" {
            agents::rig_generate::generate(agents::rig_generate::RigGenerateConfig::default())
                .await;
        }

        if input == "help" {
            println!("Commands:");
            println!("  agent   -> Voice agent (non-streaming TTS)");
            println!("  stream  -> Voice agent (streaming TTS)");
            println!("  llm     -> Local LLM chat (llama.cpp)");
            println!("  rig     -> Ollama LLM chat (rig)");
            println!("  tts     -> Text-to-speech (type text to speak)");
            println!("  stt     -> Speech-to-text (microphone)");
            println!("  voice   -> Record and playback");

            for (name, command) in &registry {
                println!("  {name} -> {}", command.usage);
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
