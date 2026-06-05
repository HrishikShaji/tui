use futures::StreamExt;
use rubato::Resampler;
use rig::agent::MultiTurnStreamItem;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::devices::mic::open_mic_stream;
use crate::devices::resampler::create_resampler;
use crate::devices::speaker::{open_output, play_samples};
use crate::llm::rig::{create_ollama_client, create_voice_agent};
use crate::stt::sherpa::{VadConfig, create_recognizer, create_vad, transcribe_audio};
use crate::tts::sherpa::{create_tts, synthesize};

const SAMPLE_RATE: i32 = 16000;
const MIC_SAMPLE_RATE: u32 = 48000;
const MIC_CHUNK_SIZE: usize = 960;

/// Speak a piece of text through the given output handle, if synthesis
/// succeeds.
fn speak_text(
    tts: &sherpa_onnx::OfflineTts,
    text: &str,
    handle: &rodio::OutputStreamHandle,
) {
    if let Some((samples, sample_rate)) = synthesize(tts, text, 1.0) {
        play_samples(handle, &samples, sample_rate);
    } else {
        eprintln!("[tts] generation failed");
    }
}

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

    println!("[agent] Initializing resampler...");
    let mut resampler = create_resampler(MIC_SAMPLE_RATE, SAMPLE_RATE as u32, MIC_CHUNK_SIZE);

    println!("[agent] Initializing TTS...");
    let tts = create_tts();

    println!("[agent] Connecting to Ollama...");
    let client = create_ollama_client().expect("failed to build Ollama client");
    let llm_agent = create_voice_agent(&client);

    println!("[agent] Opening microphone...");
    let (_stream, rx) = open_mic_stream().expect("failed to open microphone");

    // ── Main loop ────────────────────────────────────────────────────
    let is_speaking = Arc::new(AtomicBool::new(false));

    println!("\n[agent] Ready. Listening...\n");

    let mut mic_buffer: Vec<f32> = Vec::new();
    let mut current_speech: Vec<f32> = Vec::new();
    let mut was_speaking = false;

    loop {
        let chunk = rx.recv().expect("failed to receive audio");

        if is_speaking.load(Ordering::Relaxed) {
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
                println!("\n[agent] Speech detected...");

                // ── Transcribe ───────────────────────────────────────
                let transcript = match transcribe_audio(&recognizer, &current_speech, SAMPLE_RATE) {
                    Some(t) => t,
                    None => {
                        println!("[agent] empty transcript");
                        current_speech.clear();
                        was_speaking = false;
                        continue;
                    }
                };

                current_speech.clear();
                was_speaking = false;

                println!("\n[agent] {}", transcript);

                // ── Streaming LLM + sentence-by-sentence TTS ─────────
                println!("\n[agent] Generating response...\n");
                is_speaking.store(true, Ordering::Relaxed);

                let (_out_stream, handle) = open_output();
                let mut llm_stream = llm_agent.stream_prompt(&transcript).await;
                let mut sentence_buffer = String::new();

                print!("[assistant] ");
                io::stdout().flush().unwrap();

                while let Some(item) = llm_stream.next().await {
                    match item {
                        Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(t),
                        )) => {
                            print!("{}", t.text);
                            io::stdout().flush().unwrap();
                            sentence_buffer.push_str(&t.text);

                            // Extract and speak complete sentences
                            loop {
                                let pos = sentence_buffer
                                    .find(|c| c == '.' || c == '?' || c == '!' || c == '\n');

                                if let Some(idx) = pos {
                                    let sentence = sentence_buffer[..=idx].to_string();
                                    sentence_buffer = sentence_buffer[idx + 1..].to_string();

                                    let text = sentence.trim();
                                    if !text.is_empty() {
                                        println!("\n[tts] {}", text);
                                        speak_text(&tts, text, &handle);
                                    }
                                } else {
                                    break;
                                }
                            }

                            // Fallback flush for long runs without punctuation
                            if sentence_buffer.len() > 120 {
                                let text = sentence_buffer.trim().to_string();
                                sentence_buffer.clear();

                                if !text.is_empty() {
                                    speak_text(&tts, &text, &handle);
                                }
                            }
                        }

                        Ok(_) => {}

                        Err(e) => {
                            eprintln!("\n[agent] LLM error: {}", e);
                            break;
                        }
                    }
                }

                // Speak remaining buffered text
                let remaining = sentence_buffer.trim();
                if !remaining.is_empty() {
                    speak_text(&tts, remaining, &handle);
                }

                is_speaking.store(false, Ordering::Relaxed);

                // ── Cleanup ──────────────────────────────────────────
                mic_buffer.clear();
                current_speech.clear();
                vad.clear();

                println!("\n\n[agent] Done speaking.");
                println!("\n[agent] Listening...\n");
            }
        }
    }
}
