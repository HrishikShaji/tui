use futures::StreamExt;
use rubato::Resampler;
use rig::agent::MultiTurnStreamItem;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use std::io::{self, BufRead, Write};

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

pub struct NonStreamingAgentConfig {
    pub transcribe: TranscribeConfig,
    pub rig: RigGenerateConfig,
    pub tts_speed: f32,
}

impl Default for NonStreamingAgentConfig {
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
        }
    }
}

pub async fn agent_with_config(config: NonStreamingAgentConfig) {
    let mic_sample_rate = config.transcribe.mic_sample_rate;
    let mic_chunk_size = config.transcribe.mic_chunk_size;

    // ── Welcome banner ───────────────────────────────────────────────
    println!();
    println!("=== Voice Agent (Non-Streaming TTS) ===");
    println!("LLM      : Ollama {} @ {}", config.rig.model, config.rig.base_url);
    println!("Preamble : {}", config.rig.preamble);
    println!("STT      : Whisper (sherpa-onnx)");
    println!("TTS      : VITS en-lessac-medium, speed={}", config.tts_speed);
    println!("VAD      : Silero (threshold=0.7, silence=1.2s)");
    println!();
    println!("Press Enter to start a voice turn, or type 'exit' to quit.");
    println!();

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

    let tts_speed = config.tts_speed;
    let stdin = io::stdin();

    // ── Main loop ────────────────────────────────────────────────────
    loop {
        print!("agent> Press Enter to speak, or type 'exit': ");
        io::stdout().flush().unwrap();

        let mut cmd = String::new();
        match stdin.lock().read_line(&mut cmd) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        let cmd = cmd.trim();
        if cmd.eq_ignore_ascii_case("exit") || cmd.eq_ignore_ascii_case("quit") {
            println!("Leaving voice agent.");
            break;
        }

        // ── Listen for one utterance ─────────────────────────────────
        println!("[agent] Opening microphone...");
        let (_stream, rx) = open_mic_stream().expect("failed to open microphone");
        println!("[agent] Listening...");

        let mut mic_buffer: Vec<f32> = Vec::new();
        let mut current_speech: Vec<f32> = Vec::new();
        let mut was_speaking = false;
        let mut transcript_result: Option<String> = None;

        loop {
            let chunk = rx.recv().expect("failed to receive audio");
            mic_buffer.extend_from_slice(&chunk);

            while mic_buffer.len() >= mic_chunk_size {
                let input_chunk: Vec<f32> = mic_buffer.drain(..mic_chunk_size).collect();

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

                if was_speaking && vad.is_empty() && current_speech.len() > SAMPLE_RATE as usize {
                    println!("\n[agent] Speech detected, transcribing...");

                    let text = transcribe_audio(&recognizer, &current_speech, SAMPLE_RATE);
                    current_speech.clear();
                    vad.clear();

                    if let Some(t) = text {
                        transcript_result = Some(t);
                        break;
                    } else {
                        println!("[agent] (empty transcript, try again)");
                        was_speaking = false;
                    }
                }
            }

            if transcript_result.is_some() {
                break;
            }
        }

        let transcript = match transcript_result {
            Some(t) => t,
            None => continue,
        };

        println!("[agent] Transcript: \"{}\"", transcript);

        // ── LLM ──────────────────────────────────────────────────────
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
            continue;
        }

        // ── TTS ──────────────────────────────────────────────────────
        println!("[agent] Speaking response...");

        if let Some((samples, sample_rate)) = synthesize(&tts, &response_text, tts_speed) {
            let (_out_stream, handle) = open_output();
            play_samples(&handle, &samples, sample_rate);
        } else {
            eprintln!("[agent] TTS generation failed");
        }

        println!("[agent] Done speaking.\n");
    }
}

/// Convenience wrapper using default configuration.
pub async fn agent() {
    agent_with_config(NonStreamingAgentConfig::default()).await;
}
