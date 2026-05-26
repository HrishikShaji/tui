use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub fn record_and_play() -> Result<(), Box<dyn std::error::Error>> {
    // ---------------------------
    // RECORD AUDIO
    // ---------------------------
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No microphone found");
    let config = device.default_input_config()?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize; // ← use actual channel count

    println!("Device: channels={}, sample_rate={}", channels, sample_rate);

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();
    let err_fn = |err| eprintln!("Stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[f32], _| {
                let mut buf = samples_clone.lock().unwrap();
                if channels == 1 {
                    // Already mono, just extend
                    buf.extend_from_slice(data);
                } else {
                    // Downmix: average all channels per frame into one mono sample
                    for frame in data.chunks(channels) {
                        let mono = frame.iter().sum::<f32>() / channels as f32;
                        buf.push(mono);
                    }
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[i16], _| {
                let mut buf = samples_clone.lock().unwrap();
                for frame in data.chunks(channels) {
                    let mono = frame
                        .iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .sum::<f32>()
                        / channels as f32;
                    buf.push(mono);
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.clone().into(),
            move |data: &[u16], _| {
                let mut buf = samples_clone.lock().unwrap();
                for frame in data.chunks(channels) {
                    let mono = frame
                        .iter()
                        .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .sum::<f32>()
                        / channels as f32;
                    buf.push(mono);
                }
            },
            err_fn,
            None,
        )?,
        fmt => panic!("Unsupported sample format: {:?}", fmt),
    };

    println!("Recording for 5 seconds...");
    stream.play()?;
    thread::sleep(Duration::from_secs(5));
    drop(stream);

    // ---------------------------
    // SAVE TO WAV
    // ---------------------------
    let spec = hound::WavSpec {
        channels: 1, // always mono after downmix
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let recorded = samples.lock().unwrap();
    println!("Captured {} mono samples", recorded.len());

    let mut writer = hound::WavWriter::create("voice.wav", spec)?;
    for &sample in recorded.iter() {
        // Clamp to [-1.0, 1.0] to prevent clipping artifacts
        writer.write_sample(sample.clamp(-1.0, 1.0))?;
    }
    writer.finalize()?;
    println!("Saved voice.wav");

    // ---------------------------
    // PLAY AUDIO
    // ---------------------------
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let file = BufReader::new(File::open("voice.wav")?);
    let source = Decoder::new(file)?;
    sink.append(source);
    println!("Playing recording...");
    sink.sleep_until_end();

    Ok(())
}
