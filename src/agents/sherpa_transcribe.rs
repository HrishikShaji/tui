use rubato::Resampler;

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

/// Continuously listen on the default microphone, detect speech via VAD,
/// and print transcriptions.
pub fn transcribe(config: TranscribeConfig) {
    println!("[stt] Initializing recognizer...");
    let recognizer = create_recognizer();

    println!("[stt] Initializing VAD...");
    let mut vad = create_vad(config.vad);

    println!("[stt] Initializing resampler ({}Hz -> {}Hz)...", config.mic_sample_rate, SAMPLE_RATE);
    let mut resampler =
        create_resampler(config.mic_sample_rate, SAMPLE_RATE as u32, config.mic_chunk_size);

    let (_stream, rx) = open_mic_stream().expect("failed to open microphone");

    println!("Listening...");

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
                println!("Listening...");
            }
        }
    }
}
