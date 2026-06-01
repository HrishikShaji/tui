use hound;
use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig,
};

pub fn transcribe() {
    // Whisper config
    let whisper_config = OfflineWhisperModelConfig {
        encoder: Some("models/tiny.en-encoder.int8.onnx".to_string()),
        decoder: Some("models/tiny.en-decoder.int8.onnx".to_string()),

        language: Some("en".to_string()),
        task: Some("transcribe".to_string()),

        ..Default::default()
    };

    // Main model config
    let model_config = OfflineModelConfig {
        whisper: whisper_config,

        tokens: Some("models/tiny.en-tokens.txt".to_string()),

        ..Default::default()
    };

    // Recognizer config
    let config = OfflineRecognizerConfig {
        model_config,

        ..Default::default()
    };

    // Create recognizer
    let recognizer = OfflineRecognizer::create(&config).expect("failed to create recognizer");

    // Open WAV file
    let mut reader = hound::WavReader::open("test.wav").expect("failed to open wav");

    let spec = reader.spec();

    println!("Sample Rate: {}", spec.sample_rate);
    println!("Channels: {}", spec.channels);
    println!("Bits Per Sample: {}", spec.bits_per_sample);

    // Read and convert samples
    let samples: Vec<f32> = if spec.channels == 1 {
        // Mono
        reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect()
    } else {
        // Stereo -> Mono
        let all_samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();

        all_samples
            .chunks_exact(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    };

    println!("Total Samples: {}", samples.len());

    let duration = samples.len() as f32 / spec.sample_rate as f32;

    println!("Duration: {:.2} seconds", duration);

    // Create stream
    let mut stream = recognizer.create_stream();

    // Feed audio
    stream.accept_waveform(spec.sample_rate as i32, &samples);

    // Decode
    recognizer.decode(&mut stream);

    // Get result
    let result = stream.get_result().expect("no transcription result");

    println!("\nTranscript:");
    println!("{}", result.text);
}
