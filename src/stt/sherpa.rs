use rubato::Resampler;
use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig,
    SileroVadModelConfig, VadModelConfig, VoiceActivityDetector,
};

use crate::utils::model_path;

const SAMPLE_RATE: i32 = 16000;
const MIC_SAMPLE_RATE: u32 = 48000;
const MIC_CHUNK_SIZE: usize = 960;

// ─── VAD configuration ───────────────────────────────────────────────

/// Configurable parameters for the Silero VAD.
pub struct VadConfig {
    pub threshold: f32,
    pub min_silence_duration: f32,
    pub min_speech_duration: f32,
    pub window_size: i32,
    pub max_speech_duration: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            min_silence_duration: 0.8,
            min_speech_duration: 0.25,
            window_size: 512,
            max_speech_duration: 20.0,
        }
    }
}

// ─── Builder functions ───────────────────────────────────────────────

/// Create an offline Whisper recognizer for English transcription.
pub fn create_recognizer() -> OfflineRecognizer {
    let whisper_config = OfflineWhisperModelConfig {
        encoder: Some(model_path("whisper/encoder.int8.onnx")),
        decoder: Some(model_path("whisper/decoder.int8.onnx")),
        language: Some("en".to_string()),
        task: Some("transcribe".to_string()),
        ..Default::default()
    };

    let model_config = OfflineModelConfig {
        whisper: whisper_config,
        tokens: Some(model_path("whisper/tokens.txt")),
        ..Default::default()
    };

    let recognizer_config = OfflineRecognizerConfig {
        model_config,
        ..Default::default()
    };

    OfflineRecognizer::create(&recognizer_config).expect("failed to create recognizer")
}

/// Create a Silero voice-activity detector with the given configuration.
pub fn create_vad(config: VadConfig) -> VoiceActivityDetector {
    let silero_vad = SileroVadModelConfig {
        model: Some(model_path("vad/silero.onnx")),
        threshold: config.threshold,
        min_silence_duration: config.min_silence_duration,
        min_speech_duration: config.min_speech_duration,
        window_size: config.window_size,
        max_speech_duration: config.max_speech_duration,
    };

    let vad_model = VadModelConfig {
        silero_vad,
        sample_rate: SAMPLE_RATE,
        num_threads: 1,
        provider: Some("cpu".to_string()),
        debug: false,
        ..Default::default()
    };

    VoiceActivityDetector::create(&vad_model, 30.0).expect("failed to create VAD")
}

/// Run the recognizer on a buffer of 16 kHz mono samples and return the
/// trimmed transcript, or `None` if empty / inaudible.
pub fn transcribe_audio(
    recognizer: &OfflineRecognizer,
    samples: &[f32],
    sample_rate: i32,
) -> Option<String> {
    let mut stream = recognizer.create_stream();
    stream.accept_waveform(sample_rate, samples);
    recognizer.decode(&mut stream);

    match stream.get_result() {
        Some(r) => {
            let text = r.text.trim().to_string();
            if text.is_empty() || text == "[inaudible]" {
                None
            } else {
                Some(text)
            }
        }
        None => None,
    }
}

// ─── Standalone demo (called by the `stt` REPL command) ─────────────

/// Continuously listen on the default microphone, detect speech via VAD,
/// and print transcriptions. This is the standalone `stt` command.
pub fn transcribe() {
    use crate::devices::mic::open_mic_stream;
    use crate::devices::resampler::create_resampler;

    println!("[stt] Initializing recognizer...");
    let recognizer = create_recognizer();

    println!("[stt] Initializing VAD...");
    let mut vad = create_vad(VadConfig::default());

    println!("[stt] Initializing resampler (48kHz -> 16kHz)...");
    let mut resampler = create_resampler(MIC_SAMPLE_RATE, SAMPLE_RATE as u32, MIC_CHUNK_SIZE);

    let (_stream, rx) = open_mic_stream().expect("failed to open microphone");

    println!("Listening...");

    let mut mic_buffer: Vec<f32> = Vec::new();
    let mut current_speech: Vec<f32> = Vec::new();
    let mut was_speaking = false;

    loop {
        let chunk = rx.recv().expect("failed to receive audio");
        mic_buffer.extend_from_slice(&chunk);

        while mic_buffer.len() >= MIC_CHUNK_SIZE {
            let input_chunk: Vec<f32> = mic_buffer.drain(..MIC_CHUNK_SIZE).collect();

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
