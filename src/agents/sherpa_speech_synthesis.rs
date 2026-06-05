use std::io::{self, BufRead, Write};

use crate::devices::speaker::{open_output, play_samples};
use crate::tts::sherpa::{create_tts, synthesize};

// ─── Configuration ───────────────────────────────────────────────────

pub struct SpeechSynthesisConfig {
    pub speed: f32,
}

impl Default for SpeechSynthesisConfig {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

// ─── Service entry point ─────────────────────────────────────────────

/// Interactive TTS shell: accepts user text and speaks it.
pub fn speak(config: SpeechSynthesisConfig) {
    println!();
    println!("=== TTS Agent ===");
    println!("Model : VITS en-lessac-medium (ONNX)");
    println!("Speed : {}", config.speed);
    println!();
    println!("Type text to synthesize, or 'exit' to quit.");
    println!();

    let tts = create_tts();
    let (_stream, handle) = open_output();
    let stdin = io::stdin();

    loop {
        print!("tts> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Leaving TTS agent.");
            break;
        }

        match synthesize(&tts, input, config.speed) {
            Some((samples, sample_rate)) => {
                play_samples(&handle, &samples, sample_rate);
            }
            None => {
                eprintln!("[tts] Failed to synthesize audio.");
            }
        }
    }
}
