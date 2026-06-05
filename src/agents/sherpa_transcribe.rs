use rubato::Resampler;
use std::io::{self, BufRead, Write};

use crate::devices::mic::open_mic_stream;
use crate::devices::resampler::create_resampler;
use crate::stt::sherpa::{SAMPLE_RATE, VadConfig, create_recognizer, create_vad, transcribe_audio};

// ─── Configuration ───────────────────────────────────────────────────

pub struct TranscribeConfig {
    pub vad: VadConfig,
    pub mic_sample_rate: u32,
    pub mic_chunk_size: usize,
}

impl Default for TranscribeConfig {
    fn default() -> Self {
        Self {
            vad: VadConfig::default(),
            mic_sample_rate: 48000,
            mic_chunk_size: 960,
        }
    }
}

// ─── Service entry point ─────────────────────────────────────────────

/// Interactive STT shell: listens for speech and prints transcriptions.
pub fn transcribe(config: TranscribeConfig) {
    println!();
    println!("=== STT Agent ===");
    println!("Model      : Whisper (sherpa-onnx)");
    println!("VAD        : Silero (threshold={}, silence={}s, speech={}s)",
        config.vad.threshold, config.vad.min_silence_duration, config.vad.min_speech_duration);
    println!("Mic        : {}Hz, chunk size {}", config.mic_sample_rate, config.mic_chunk_size);
    println!();
    println!("Press Enter to start listening, or type 'exit' to quit.");
    println!();

    println!("[stt] Initializing recognizer...");
    let recognizer = create_recognizer();

    println!("[stt] Initializing VAD...");
    let mut vad = create_vad(config.vad);

    println!("[stt] Initializing resampler ({}Hz -> {}Hz)...", config.mic_sample_rate, SAMPLE_RATE);
    let mut resampler =
        create_resampler(config.mic_sample_rate, SAMPLE_RATE as u32, config.mic_chunk_size);

    let stdin = io::stdin();

    loop {
        print!("stt> Press Enter to listen, or type 'exit': ");
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
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Leaving STT agent.");
            break;
        }

        let (_stream, rx) = open_mic_stream().expect("failed to open microphone");

        println!("Listening... (will stop after detecting speech + silence)");

        let mut mic_buffer: Vec<f32> = Vec::new();
        let mut current_speech: Vec<f32> = Vec::new();
        let mut was_speaking = false;

        loop {
            let chunk = rx.recv().expect("failed to receive audio");
            mic_buffer.extend_from_slice(&chunk);

            while mic_buffer.len() >= config.mic_chunk_size {
                let input_chunk: Vec<f32> = mic_buffer.drain(..config.mic_chunk_size).collect();

                let resampled = resampler
                    .process(&vec![input_chunk], None)
                    .expect("resampling failed");
                let chunk_16k = &resampled[0];

                vad.accept_waveform(chunk_16k);

                while !vad.is_empty() {
                    let segment = vad.front().unwrap();
                    current_speech.extend_from_slice(segment.samples());
                    was_speaking = true;
                    vad.pop();
                }

                if was_speaking && current_speech.len() > SAMPLE_RATE as usize {
                    println!("Processing speech...");

                    if let Some(text) = transcribe_audio(&recognizer, &current_speech, SAMPLE_RATE) {
                        println!("Transcript: {}", text);
                    }

                    current_speech.clear();
                    was_speaking = false;
                    vad.clear();

                    // Break out of the inner listen loop after one transcription
                    break;
                }
            }

            // If we just transcribed, break the outer recv loop too
            if !was_speaking && current_speech.is_empty() && mic_buffer.is_empty() {
                break;
            }
        }

        println!();
    }
}
