use llama_cpp_2::context::params::LlamaContextParams;
use std::io::{self, BufRead, Write};

use crate::llm::llama::{build_prompt, generate_response, load_model};
use crate::utils::model_path;

// ─── Configuration ───────────────────────────────────────────────────

pub struct LlamaGenerateConfig {
    pub model_file: String,
    pub max_tokens: usize,
}

impl Default for LlamaGenerateConfig {
    fn default() -> Self {
        Self {
            model_file: model_path("llm/llama-3.2.gguf"),
            max_tokens: 512,
        }
    }
}

// ─── Service entry point ─────────────────────────────────────────────

/// Interactive local LLM chat loop using llama.cpp.
pub fn generate(config: LlamaGenerateConfig) {
    println!();
    println!("=== LLM Agent (llama.cpp) ===");
    println!("Model      : {}", config.model_file);
    println!("Max tokens : {}", config.max_tokens);
    println!();
    println!("Type your message, or 'exit' to quit.");
    println!();

    let (backend, model) = load_model(&config.model_file);

    let stdin = io::stdin();
    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        print!("llm> ");
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

        let user_input = user_input.trim().to_string();
        if user_input.is_empty() {
            continue;
        }
        if user_input.eq_ignore_ascii_case("exit") || user_input.eq_ignore_ascii_case("quit") {
            println!("Leaving LLM agent.");
            break;
        }

        let prompt = build_prompt(&history, &user_input);

        let ctx_params = LlamaContextParams::default();
        let mut ctx = model
            .new_context(&backend, ctx_params)
            .expect("Failed to create context");

        print!("Assistant: ");
        io::stdout().flush().unwrap();

        let assistant_response = generate_response(&mut ctx, &model, &prompt, config.max_tokens);
        history.push((user_input, assistant_response.trim().to_string()));
    }
}
