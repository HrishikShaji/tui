use llama_cpp_2::{LogOptions, send_logs_to_tracing};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
    sampling::LlamaSampler,
};
use std::io::{self, BufRead, Write};

use crate::utils::model_path;

const STOP_PATTERNS: &[&str] = &[
    "\nUser:",
    "\nUser\n",
    "\nUser ",
    "\nAssistant:",
    "\nAssistant\n",
];

fn find_stop_cutoff(response: &str) -> Option<usize> {
    STOP_PATTERNS
        .iter()
        .filter_map(|pat| response.find(pat))
        .min()
}

// ─── Builder functions ───────────────────────────────────────────────

/// Initialise the llama.cpp backend and load the local GGUF model.
/// Returns `(backend, model)` for use by callers that want to manage
/// context creation themselves.
pub fn load_model() -> (LlamaBackend, LlamaModel) {
    unsafe {
        std::env::set_var("LLAMA_CPP_LOG_LEVEL", "0");
    }

    let log_options = LogOptions::default().with_logs_enabled(false);
    send_logs_to_tracing(log_options);

    let backend = LlamaBackend::init().expect("failed to init llama backend");

    let model_params = LlamaModelParams::default();
    let path = model_path("llm/llama-3.2.gguf");
    let model =
        LlamaModel::load_from_file(&backend, &path, &model_params).expect("failed to load model");

    (backend, model)
}

// ─── Generation ──────────────────────────────────────────────────────

/// Generate a response by feeding `prompt` into the model, streaming
/// tokens to stdout. Returns the full generated text.
pub fn generate_response(
    ctx: &mut llama_cpp_2::context::LlamaContext,
    model: &LlamaModel,
    prompt: &str,
    max_tokens: usize,
) -> String {
    let tokens = match model.str_to_token(prompt, AddBos::Always) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to tokenize prompt: {e}");
            return String::new();
        }
    };

    let mut batch = LlamaBatch::new(4096, 1);
    let last_idx = tokens.len() - 1;
    for (i, &token) in tokens.iter().enumerate() {
        batch
            .add(token, i as i32, &[0], i == last_idx)
            .expect("Failed to add token to batch");
    }
    ctx.decode(&mut batch)
        .expect("Failed to decode prompt batch");

    let mut sampler =
        LlamaSampler::chain_simple([LlamaSampler::dist(1234), LlamaSampler::greedy()]);

    let mut n_decoded = 0;
    let mut n_pos = tokens.len() as i32;
    let mut response = String::new();

    loop {
        let token = sampler.sample(ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) || n_decoded >= max_tokens {
            break;
        }

        let token_str = model
            .token_to_str(token, Special::Tokenize)
            .expect("Failed to decode token");

        response.push_str(&token_str);

        if let Some(cutoff) = find_stop_cutoff(&response) {
            let clean = response[..cutoff].to_string();
            let already_printed = response.len() - token_str.len();
            if cutoff > already_printed {
                print!("{}", &clean[already_printed..]);
            }
            io::stdout().flush().unwrap();
            println!();
            return clean;
        }

        print!("{token_str}");
        io::stdout().flush().unwrap();

        batch.clear();
        batch
            .add(token, n_pos, &[0], true)
            .expect("Failed to add token");
        n_pos += 1;
        n_decoded += 1;
        ctx.decode(&mut batch).expect("Failed to decode token");
    }

    println!();
    response
}

/// Build a multi-turn prompt string from conversation history and the
/// current user input.
pub fn build_prompt(history: &[(String, String)], user_input: &str) -> String {
    let mut prompt = String::new();
    for (user, assistant) in history {
        prompt.push_str(&format!("User: {user}\nAssistant: {assistant}\n"));
    }
    prompt.push_str(&format!("User: {user_input}\nAssistant:"));
    prompt
}

// ─── Standalone REPL (called by the `llm` REPL command) ─────────────

/// Interactive local LLM chat loop. This is the standalone `llm` command.
pub fn run_local_agent() {
    let (backend, model) = load_model();

    println!("Model loaded. Type your message and press Enter (Ctrl+C or 'exit' to quit):\n");

    let stdin = io::stdin();
    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        print!("You: ");
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
            println!("Goodbye!");
            break;
        }

        let prompt = build_prompt(&history, &user_input);

        let ctx_params = LlamaContextParams::default();
        let mut ctx = model
            .new_context(&backend, ctx_params)
            .expect("Failed to create context");

        print!("Assistant: ");
        io::stdout().flush().unwrap();

        let assistant_response = generate_response(&mut ctx, &model, &prompt, 512);
        history.push((user_input, assistant_response.trim().to_string()));
    }
}
