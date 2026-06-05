use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsModelConfig,
    OfflineTtsVitsModelConfig,
};

use crate::utils::model_path;

// ─── Builder functions ───────────────────────────────────────────────

/// Create an offline VITS text-to-speech engine.
pub fn create_tts() -> OfflineTts {
    let vits = OfflineTtsVitsModelConfig {
        model: Some(model_path("tts/en-lessac-medium.onnx")),
        tokens: Some(model_path("tts/tokens.txt")),
        data_dir: Some(model_path("tts/espeak-ng-data")),
        ..Default::default()
    };

    let model = OfflineTtsModelConfig {
        vits,
        provider: Some("cpu".to_string()),
        num_threads: 1,
        debug: false,
        ..Default::default()
    };

    let config = OfflineTtsConfig {
        model,
        max_num_sentences: 1,
        rule_fsts: Some("".to_string()),
        rule_fars: Some("".to_string()),
        silence_scale: 0.2,
    };

    OfflineTts::create(&config).expect("failed to create TTS")
}

/// Synthesize speech from text. Returns `(samples, sample_rate)` or `None`
/// if generation fails.
pub fn synthesize(tts: &OfflineTts, text: &str, speed: f32) -> Option<(Vec<f32>, u32)> {
    let gen_config = GenerationConfig {
        speed,
        ..Default::default()
    };

    tts.generate_with_config(text, &gen_config, None::<fn(&[f32], f32) -> bool>)
        .map(|audio| (audio.samples().to_vec(), audio.sample_rate() as u32))
}


