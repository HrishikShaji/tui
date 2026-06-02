use crate::voice::stt::to_mono;

use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{Client, CompletionClient, Nothing};
use rig::providers::ollama;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};

use rodio::{OutputStream, Sink, buffer::SamplesBuffer};

use sherpa_onnx::{
    GenerationConfig, OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineTts,
    OfflineTtsConfig, OfflineTtsModelConfig, OfflineTtsVitsModelConfig, OfflineWhisperModelConfig,
    SileroVadModelConfig, VadModelConfig, VoiceActivityDetector,
};

use std::io::{self, Write};

const SAMPLE_RATE: i32 = 16000;
const MIC_SAMPLE_RATE: u32 = 48000;
const MIC_CHUNK_SIZE: usize = 960;

pub async fn agent() {
    // =========================================
    // BUILD WHISPER RECOGNIZER
    // =========================================
    println!("[agent] Initializing speech recognizer...");

    let whisper_config = OfflineWhisperModelConfig {
        encoder: Some("models/tiny.en-encoder.int8.onnx".to_string()),
        decoder: Some("models/tiny.en-decoder.int8.onnx".to_string()),
        language: Some("en".to_string()),
        task: Some("transcribe".to_string()),
        ..Default::default()
    };

    let model_config = OfflineModelConfig {
        whisper: whisper_config,
        tokens: Some("models/tiny.en-tokens.txt".to_string()),
        ..Default::default()
    };

    let recognizer_config = OfflineRecognizerConfig {
        model_config,
        ..Default::default()
    };

    let recognizer =
        OfflineRecognizer::create(&recognizer_config).expect("failed to create recognizer");

    // =========================================
    // BUILD VAD
    // =========================================
    println!("[agent] Initializing VAD...");

    let silero_vad = SileroVadModelConfig {
        model: Some("models/silero_vad.onnx".to_string()),

        // More aggressive settings
        threshold: 0.7,
        min_silence_duration: 1.2,
        min_speech_duration: 0.4,

        window_size: 512,
        max_speech_duration: 20.0,
    };

    let vad_model = VadModelConfig {
        silero_vad,
        sample_rate: SAMPLE_RATE,
        num_threads: 1,
        provider: Some("cpu".to_string()),
        debug: false,
        ..Default::default()
    };

    let mut vad = VoiceActivityDetector::create(&vad_model, 30.0).expect("failed to create VAD");

    // =========================================
    // BUILD RESAMPLER
    // =========================================
    println!("[agent] Initializing resampler (48kHz -> 16kHz)...");

    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    let resample_ratio = SAMPLE_RATE as f64 / MIC_SAMPLE_RATE as f64;

    let sinc_params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler =
        SincFixedIn::<f32>::new(resample_ratio, 2.0, sinc_params, MIC_CHUNK_SIZE, 1)
            .expect("failed to create resampler");

    // =========================================
    // BUILD TTS
    // =========================================
    println!("[agent] Initializing TTS...");

    let vits = OfflineTtsVitsModelConfig {
        model: Some("models/en_US-lessac-medium.onnx".to_string()),
        tokens: Some("models/tokens.txt".to_string()),
        data_dir: Some("models/espeak-ng-data".to_string()),
        ..Default::default()
    };

    let tts_model = OfflineTtsModelConfig {
        vits,
        provider: Some("cpu".to_string()),
        num_threads: 1,
        debug: false,
        ..Default::default()
    };

    let tts_config = OfflineTtsConfig {
        model: tts_model,
        max_num_sentences: 1,
        rule_fsts: Some("".to_string()),
        rule_fars: Some("".to_string()),
        silence_scale: 0.2,
    };

    let tts = OfflineTts::create(&tts_config).expect("failed to create TTS");

    // =========================================
    // BUILD LLM AGENT
    // =========================================
    println!("[agent] Connecting to Ollama...");

    let client = Client::<ollama::OllamaExt>::builder()
        .api_key(Nothing)
        .base_url("http://localhost:11434")
        .build()
        .expect("failed to build Ollama client");

    let llm_agent = client
        .agent("llama3.2")
        .preamble(
            "You are a helpful voice assistant. \
             Keep responses concise and conversational.",
        )
        .build();

    // =========================================
    // MICROPHONE
    // =========================================
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let (_stream, rx) = {
        let host = cpal::default_host();

        let device = host.default_input_device().expect("no microphone found");

        println!("[agent] Using microphone: {}", device.name().unwrap());

        let supported_config = device.default_input_config().unwrap();

        let sample_format = supported_config.sample_format();

        let config: cpal::StreamConfig = supported_config.into();

        println!(
            "[agent] Sample rate: {}Hz, Channels: {}",
            config.sample_rate.0, config.channels
        );

        let channels = config.channels as usize;

        let (tx, rx) = crossbeam_channel::unbounded::<Vec<f32>>();

        let err_fn = |err| eprintln!("[agent] stream error: {}", err);

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let mono = to_mono(data, channels);

                        tx.send(mono).ok();
                    },
                    err_fn,
                    None,
                )
                .unwrap(),

            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let samples: Vec<f32> = data.iter().map(|s| *s as f32 / 32768.0).collect();

                        let mono = to_mono(&samples, channels);

                        tx.send(mono).ok();
                    },
                    err_fn,
                    None,
                )
                .unwrap(),

            _ => panic!("unsupported sample format"),
        };

        stream.play().unwrap();

        (stream, rx)
    };

    // =========================================
    // MAIN LOOP
    // =========================================
    println!("[agent] Ready. Listening for speech...\n");

    let mut mic_buffer: Vec<f32> = Vec::new();

    let mut current_speech: Vec<f32> = Vec::new();

    let mut was_speaking = false;

    // Prevent microphone from hearing TTS
    let mut is_speaking = false;

    loop {
        let chunk = rx.recv().expect("failed to receive audio");

        // Ignore microphone while TTS is speaking
        if is_speaking {
            continue;
        }

        mic_buffer.extend_from_slice(&chunk);

        while mic_buffer.len() >= MIC_CHUNK_SIZE {
            let input_chunk: Vec<f32> = mic_buffer.drain(..MIC_CHUNK_SIZE).collect();

            // =====================================
            // RESAMPLE 48kHz -> 16kHz
            // =====================================
            let resampled = resampler
                .process(&vec![input_chunk], None)
                .expect("resampling failed");

            let chunk_16k = &resampled[0];

            // =====================================
            // FEED INTO VAD
            // =====================================
            vad.accept_waveform(chunk_16k);

            // Pull detected speech segments
            while !vad.is_empty() {
                let segment = vad.front().unwrap();

                current_speech.extend_from_slice(segment.samples());

                was_speaking = true;

                vad.pop();
            }

            // =====================================
            // PROCESS AFTER SILENCE
            // =====================================
            if was_speaking && vad.is_empty() && current_speech.len() > SAMPLE_RATE as usize {
                println!("\n[agent] 🎤 Speech detected, transcribing...");

                // =================================
                // TRANSCRIBE
                // =================================
                let mut stream = recognizer.create_stream();

                stream.accept_waveform(SAMPLE_RATE, &current_speech);

                recognizer.decode(&mut stream);

                let transcript = match stream.get_result() {
                    Some(r) => r.text.trim().to_string(),
                    None => String::new(),
                };

                current_speech.clear();
                was_speaking = false;

                // Skip empty transcripts
                if transcript.is_empty() || transcript == "[inaudible]" {
                    println!("[agent] (empty transcript, skipping)");

                    println!("\n[agent] Listening...");

                    continue;
                }

                println!("[agent] 📝 Transcript: \"{}\"", transcript);

                // =================================
                // LLM
                // =================================
                println!("[agent] 🤖 Sending to LLM...");

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

                    println!("\n[agent] Listening...");

                    continue;
                }

                // =================================
                // TTS
                // =================================
                println!("[agent] 🔊 Speaking response...");

                is_speaking = true;

                let gen_config = GenerationConfig {
                    speed: 1.0,
                    ..Default::default()
                };

                let audio = tts
                    .generate_with_config(
                        &response_text,
                        &gen_config,
                        None::<fn(&[f32], f32) -> bool>,
                    )
                    .expect("TTS generation failed");

                let (_out_stream, stream_handle) =
                    OutputStream::try_default().expect("failed to open audio output");

                let sink = Sink::try_new(&stream_handle).expect("failed to create sink");

                let source =
                    SamplesBuffer::new(1, audio.sample_rate() as u32, audio.samples().to_vec());

                sink.append(source);

                // Wait until speech finishes
                sink.sleep_until_end();

                // Re-enable microphone
                is_speaking = false;

                // Clear old audio
                mic_buffer.clear();
                current_speech.clear();

                vad.clear();

                println!("[agent] ✅ Done speaking.");

                println!("\n[agent] Listening...");
            }
        }
    }
}
