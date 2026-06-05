use futures::StreamExt;
use rubato::Resampler;
use rig::agent::MultiTurnStreamItem;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use std::io::{self, Write};

use crate::devices::mic::open_mic_stream;
use crate::devices::resampler::create_resampler;
use crate::devices::speaker::{open_output, play_samples};
use crate::llm::rig::{create_ollama_client, create_voice_agent};
use crate::stt::sherpa::{VadConfig, create_recognizer, create_vad, transcribe_audio};
use crate::tts::sherpa::{create_tts, synthesize};

const SAMPLE_RATE: i32 = 16000;
const MIC_SAMPLE_RATE: u32 = 48000;
const MIC_CHUNK_SIZE: usize = 960;

pub async fn agent() {
    // ── Initialise components ────────────────────────────────────────
    println!("[agent] Initializing speech recognizer...");
    let recognizer = create_recognizer();

    println!("[agent] Initializing VAD...");
    let mut vad = create_vad(VadConfig {
        threshold: 0.7,
        min_silence_duration: 1.2,
        min_speech_duration: 0.4,
        ..Default::default()
    });

    println!("[agent] Initializing resampler (48kHz -> 16kHz)...");
    let mut resampler = create_resampler(MIC_SAMPLE_RATE, SAMPLE_RATE as u32, MIC_CHUNK_SIZE);

    println!("[agent] Initializing TTS...");
    let tts = create_tts();

    println!("[agent] Connecting to Ollama...");
    let client = create_ollama_client().expect("failed to build Ollama client");
    let llm_agent = create_voice_agent(&client);

    println!("[agent] Opening microphone...");
    let (_stream, rx) = open_mic_stream().expect("failed to open microphone");

    // ── Main loop ────────────────────────────────────────────────────
    println!("[agent] Ready. Listening for speech...\n");

    let mut mic_buffer: Vec<f32> = Vec::new();
    let mut current_speech: Vec<f32> = Vec::new();
    let mut was_speaking = false;
    let mut is_speaking = false;

    loop {
        let chunk = rx.recv().expect("failed to receive audio");

        if is_speaking {
            continue;
        }

        mic_buffer.extend_from_slice(&chunk);

        while mic_buffer.len() >= MIC_CHUNK_SIZE {
            let input_chunk: Vec<f32> = mic_buffer.drain(..MIC_CHUNK_SIZE).collect();

            // Resample 48kHz -> 16kHz
            let resampled = resampler
                .process(&vec![input_chunk], None)
                .expect("resampling failed");
            let chunk_16k = &resampled[0];

            // Feed into VAD
            vad.accept_waveform(chunk_16k);

            while !vad.is_empty() {
                let segment = vad.front().unwrap();
                current_speech.extend_from_slice(segment.samples());
                was_speaking = true;
                vad.pop();
            }

            // Process after silence
            if was_speaking && vad.is_empty() && current_speech.len() > SAMPLE_RATE as usize {
                println!("\n[agent] Speech detected, transcribing...");

                // ── Transcribe ───────────────────────────────────────
                let transcript = match transcribe_audio(&recognizer, &current_speech, SAMPLE_RATE) {
                    Some(t) => t,
                    None => {
                        println!("[agent] (empty transcript, skipping)");
                        println!("\n[agent] Listening...");
                        current_speech.clear();
                        was_speaking = false;
                        continue;
                    }
                };

                current_speech.clear();
                was_speaking = false;

                println!("[agent] Transcript: \"{}\"", transcript);

                // ── LLM ──────────────────────────────────────────────
                println!("[agent] Sending to LLM...");
                print!("[agent] Response: ");
                io::stdout().flush().unwrap();

                let mut response_text = String::new();
                let mut llm_stream = llm_agent.stream_prompt(&transcript).await;

                while let Some(item) = llm_stream.next().await {
                    match item {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(t),
                        )) => {
                            print!("{}", t.text);
                            io::stdout().flush().unwrap();
                            response_text.push_str(&t.text);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("\n[agent] LLM error: {}", e);
                            break;
                        }
                    }
                }

                println!();

                if response_text.trim().is_empty() {
                    println!("[agent] (empty LLM response)");
                    println!("\n[agent] Listening...");
                    continue;
                }

                // ── TTS ──────────────────────────────────────────────
                println!("[agent] Speaking response...");
                is_speaking = true;

                if let Some((samples, sample_rate)) = synthesize(&tts, &response_text, 1.0) {
                    let (_out_stream, handle) = open_output();
                    play_samples(&handle, &samples, sample_rate);
                } else {
                    eprintln!("[agent] TTS generation failed");
                }

                is_speaking = false;

                // ── Cleanup ──────────────────────────────────────────
                mic_buffer.clear();
                current_speech.clear();
                vad.clear();

                println!("[agent] Done speaking.");
                println!("\n[agent] Listening...");
            }
        }
    }
}
