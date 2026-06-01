use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
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

    // Get audio samples
    let samples: Vec<f32> = audio.samples().to_vec();

    // Open default audio device
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("failed to open audio output");

    // Create sink
    let sink = Sink::try_new(&stream_handle).expect("failed to create sink");

    // Create source
    let source = SamplesBuffer::new(
        1, // mono
        audio.sample_rate() as u32,
        samples,
    );

    // Play
    sink.append(source);

    // Wait until finished
    sink.sleep_until_end();

    println!("Done speaking!");
}
