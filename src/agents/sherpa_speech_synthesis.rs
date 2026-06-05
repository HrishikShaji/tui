use crate::devices::speaker::{open_output, play_samples};
use crate::tts::sherpa::{create_tts, synthesize};

// ─── Configuration ───────────────────────────────────────────────────

pub struct SpeechSynthesisConfig {
    pub text: String,
    pub speed: f32,
}

impl Default for SpeechSynthesisConfig {
    fn default() -> Self {
        Self {
            text: "Hello from Sherpa ONNX text to speech in Rust.".to_string(),
            speed: 1.0,
        }
    }
}

// ─── Service entry point ─────────────────────────────────────────────

/// Synthesize the given text and play it through the default speaker.
pub fn speak(config: SpeechSynthesisConfig) {
    let tts = create_tts();

    let (samples, sample_rate) =
        synthesize(&tts, &config.text, config.speed).expect("failed to generate audio");

    let (_stream, handle) = open_output();
    play_samples(&handle, &samples, sample_rate);

    println!("Done speaking!");
}
