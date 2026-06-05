use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig,
    SileroVadModelConfig, VadModelConfig, VoiceActivityDetector,
};

use crate::utils::model_path;

pub const SAMPLE_RATE: i32 = 16000;

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


