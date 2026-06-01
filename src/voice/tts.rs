use hound;
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsModelConfig,
    OfflineTtsVitsModelConfig,
};

pub fn speak() {
    // VITS config
    let vits = OfflineTtsVitsModelConfig {
        model: Some("models/en_US-lessac-medium.onnx".to_string()),

        tokens: Some("models/tokens.txt".to_string()),

        data_dir: Some("models/espeak-ng-data".to_string()),

        ..Default::default()
    };

    // Model config
    let model = OfflineTtsModelConfig {
        vits,

        provider: Some("cpu".to_string()),

        num_threads: 1,

        debug: false,

        ..Default::default()
    };

    // Full config
    let config = OfflineTtsConfig {
        model,

        max_num_sentences: 1,

        rule_fsts: Some("".to_string()),

        rule_fars: Some("".to_string()),

        silence_scale: 0.2,
    };

    // Create TTS
    let tts = OfflineTts::create(&config).expect("failed to create tts");

    let text = "Hello from Sherpa ONNX text to speech in Rust.";

    // Generation config
    let gen_config = GenerationConfig {
        speed: 1.0,

        ..Default::default()
    };

    // Generate audio
    let audio = tts
        .generate_with_config(text, &gen_config, None::<fn(&[f32], f32) -> bool>)
        .expect("failed to generate audio");

    // WAV spec
    let spec = hound::WavSpec {
        channels: 1,

        sample_rate: audio.sample_rate() as u32,

        bits_per_sample: 16,

        sample_format: hound::SampleFormat::Int,
    };

    // Create writer
    let mut writer = hound::WavWriter::create("output1.wav", spec).expect("failed to create wav");

    // Write samples
    for sample in audio.samples() {
        let s = (sample * i16::MAX as f32) as i16;

        writer.write_sample(s).unwrap();
    }

    writer.finalize().unwrap();

    println!("Saved output.wav");
}
