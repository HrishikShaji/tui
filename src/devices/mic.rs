use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, unbounded};

/// Convert multi-channel audio to mono by averaging all channels per frame.
pub fn to_mono(input: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return input.to_vec();
    }

    input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Open the default microphone and return a stream + a receiver that yields
/// mono `f32` sample chunks. The caller must keep `cpal::Stream` alive for
/// the duration of recording.
pub fn open_mic_stream() -> Result<(cpal::Stream, Receiver<Vec<f32>>)> {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no microphone found");

    println!("[mic] Using device: {}", device.name().unwrap());

    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();

    println!(
        "[mic] Sample rate: {}Hz, Channels: {}",
        config.sample_rate.0, config.channels
    );

    let channels = config.channels as usize;
    let (tx, rx) = unbounded::<Vec<f32>>();

    let err_fn = |err| eprintln!("[mic] stream error: {}", err);

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
