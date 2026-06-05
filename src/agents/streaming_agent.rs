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
use crate::stt::sherpa::{SAMPLE_RATE, VadConfig, create_recognizer, create_vad, transcribe_audio};
use crate::tts::sherpa::{create_tts, synthesize};

use super::rig_generate::RigGenerateConfig;
use super::sherpa_transcribe::TranscribeConfig;

const MIC_SAMPLE_RATE: u32 = 48000;
const MIC_CHUNK_SIZE: usize = 960;

fn agent_vad() -> VadConfig {
    VadConfig {
        threshold: 0.7,
        min_silence_duration: 1.2,
        min_speech_duration: 0.4,
        window_size: 512,
        max_speech_duration: 20.0,
    }
}

pub struct StreamingAgentConfig {
    pub transcribe: TranscribeConfig,
    pub rig: RigGenerateConfig,
    pub tts_speed: f32,
    pub sentence_buffer_flush: usize,
}

impl Default for StreamingAgentConfig {
    fn default() -> Self {
        Self {
            transcribe: TranscribeConfig {
                vad: agent_vad(),
                mic_sample_rate: MIC_SAMPLE_RATE,
                mic_chunk_size: MIC_CHUNK_SIZE,
            },
            rig: RigGenerateConfig {
                preamble: "You are a helpful voice assistant. \
                           Keep responses concise and conversational."
                    .to_string(),
                ..RigGenerateConfig::default()
            },
            tts_speed: 1.0,
            sentence_buffer_flush: 120,
        }
    }
}

/// Speak a piece of text through the given output handle, if synthesis
/// succeeds.
fn speak_text(
    tts: &sherpa_onnx::OfflineTts,
    text: &str,
    speed: f32,
    handle: &rodio::OutputStreamHandle,
) {
    if let Some((samples, sample_rate)) = synthesize(tts, text, speed) {
        play_samples(handle, &samples, sample_rate);
    } else {
        eprintln!("[tts] generation failed");
    }
}

pub async fn agent_with_config(config: StreamingAgentConfig) {
    let mic_sample_rate = config.transcribe.mic_sample_rate;
    let mic_chunk_size = config.transcribe.mic_chunk_size;
    let tts_speed = config.tts_speed;
    let sentence_buffer_flush = config.sentence_buffer_flush;

    // ── Initialise components ────────────────────────────────────────
    println!("[agent] Initializing speech recognizer...");
    let recognizer = create_recognizer();

    println!("[agent] Initializing VAD...");
    let mut vad = create_vad(config.transcribe.vad);

    println!("[agent] Initializing resampler ({}Hz -> {}Hz)...", mic_sample_rate, SAMPLE_RATE);
    let mut resampler = create_resampler(mic_sample_rate, SAMPLE_RATE as u32, mic_chunk_size);

    println!("[agent] Initializing TTS...");
    let tts = create_tts();

    println!("[agent] Connecting to Ollama...");
    let client =
        create_ollama_client(&config.rig.base_url).expect("failed to build Ollama client");
    let llm_agent = create_voice_agent(&client, &config.rig.model, &config.rig.preamble);

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

        while mic_buffer.len() >= mic_chunk_size {
            let input_chunk: Vec<f32> = mic_buffer.drain(..mic_chunk_size).collect();

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
                                        speak_text(&tts, text, tts_speed, &handle);
                                    }
                                } else {
                                    break;
                                }
                            }

                            // Fallback flush for long runs without punctuation
                            if sentence_buffer.len() > sentence_buffer_flush {
                                let text = sentence_buffer.trim().to_string();
                                sentence_buffer.clear();

                                if !text.is_empty() {
                                    speak_text(&tts, &text, tts_speed, &handle);
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
                    speak_text(&tts, remaining, tts_speed, &handle);
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

/// Convenience wrapper using default configuration.
pub async fn agent() {
    agent_with_config(StreamingAgentConfig::default()).await;
}
