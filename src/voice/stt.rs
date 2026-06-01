use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, unbounded};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig,
    SileroVadModelConfig, VadModelConfig, VoiceActivityDetector,
};

const SAMPLE_RATE: i32 = 16000;
const MIC_SAMPLE_RATE: u32 = 48000;
// 20ms chunks at 48kHz
const MIC_CHUNK_SIZE: usize = 960;

pub fn transcribe() {
    // =========================================
    // WHISPER CONFIG
    // =========================================

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
    // VAD CONFIG
    // =========================================

    let silero_vad = SileroVadModelConfig {
        model: Some("models/silero_vad.onnx".to_string()),
        threshold: 0.5,
        min_silence_duration: 0.8,
        min_speech_duration: 0.25,
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
    // RESAMPLER: 48000 Hz -> 16000 Hz
    // =========================================

    let resample_ratio = SAMPLE_RATE as f64 / MIC_SAMPLE_RATE as f64; // 1/3

    let sinc_params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        resample_ratio,
        2.0,
        sinc_params,
        MIC_CHUNK_SIZE,
        1, // mono
    )
    .expect("failed to create resampler");

    // =========================================
    // MICROPHONE
    // =========================================

    let (_stream, rx) = microphone_stream().expect("failed to create microphone stream");

    println!("Listening...");

    let mut current_speech: Vec<f32> = Vec::new();
    let mut was_speaking = false;
    // Buffer for accumulating mic samples into exact MIC_CHUNK_SIZE chunks
    let mut mic_buffer: Vec<f32> = Vec::new();

    loop {
        let chunk = rx.recv().expect("failed to receive audio");

        mic_buffer.extend_from_slice(&chunk);

        // Process in fixed-size chunks so the resampler always gets exactly
        // MIC_CHUNK_SIZE samples
        while mic_buffer.len() >= MIC_CHUNK_SIZE {
            let input_chunk: Vec<f32> = mic_buffer.drain(..MIC_CHUNK_SIZE).collect();

            // Resample 48kHz -> 16kHz
            let waves_in = vec![input_chunk];
            let resampled = resampler
                .process(&waves_in, None)
                .expect("resampling failed");
            let chunk_16k = &resampled[0];

            // Feed resampled audio into VAD
            vad.accept_waveform(chunk_16k);

            // Collect detected speech segments
            while !vad.is_empty() {
                let segment = vad.front().unwrap();
                current_speech.extend_from_slice(segment.samples());
                was_speaking = true;
                vad.pop();
            }

            // Speech ended -> transcribe once we have at least 1 second of audio
            if was_speaking && current_speech.len() > SAMPLE_RATE as usize {
                println!("Processing speech...");

                let mut stream = recognizer.create_stream();
                stream.accept_waveform(SAMPLE_RATE, &current_speech);
                recognizer.decode(&mut stream);

                if let Some(result) = stream.get_result() {
                    let text = result.text.trim();
                    if !text.is_empty() {
                        println!("Transcript: {}", text);
                    }
                }

                current_speech.clear();
                was_speaking = false;
                println!("Listening...");
            }
        }
    }
}

fn microphone_stream() -> Result<(cpal::Stream, Receiver<Vec<f32>>)> {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no microphone found");

    println!("Using microphone: {}", device.name().unwrap());

    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();

    println!("Input Sample Rate: {}", config.sample_rate.0);
    println!("Input Channels: {}", config.channels);

    let channels = config.channels as usize;
    let (tx, rx) = unbounded::<Vec<f32>>();

    let err_fn = |err| {
        eprintln!("stream error: {}", err);
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mono = to_mono(data, channels);
                tx.send(mono).ok();
            },
            err_fn,
            None,
        )?,

        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                let samples: Vec<f32> = data.iter().map(|s| *s as f32 / 32768.0).collect();
                let mono = to_mono(&samples, channels);
                tx.send(mono).ok();
            },
            err_fn,
            None,
        )?,

        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                let samples: Vec<f32> = data.iter().map(|s| *s as f32 / 65535.0 - 0.5).collect();
                let mono = to_mono(&samples, channels);
                tx.send(mono).ok();
            },
            err_fn,
            None,
        )?,

        _ => panic!("unsupported sample format"),
    };

    stream.play()?;
    Ok((stream, rx))
}

fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return input.to_vec();
    }

    input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
