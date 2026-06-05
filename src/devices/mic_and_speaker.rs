use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{Decoder, OutputStream, Sink};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub fn record_and_play() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No microphone found");
    let mic_config = device.default_input_config()?;

    let sample_rate = mic_config.sample_rate().0;
    let channels = mic_config.channels() as usize;

    println!();
    println!("=== Voice Recorder ===");
    println!("Device   : {} (channels={}, sample_rate={})",
        device.name().unwrap_or_else(|_| "unknown".into()), channels, sample_rate);
    println!("Duration : 5 seconds per recording");
    println!("Output   : voice.wav");
    println!();
    println!("Press Enter to record, or type 'exit' to quit.");
    println!();

    let stdin = io::stdin();

    loop {
        print!("voice> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Leaving voice recorder.");
            break;
        }

        // ---------------------------
        // RECORD AUDIO
        // ---------------------------
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = samples.clone();
        let err_fn = |err| eprintln!("Stream error: {}", err);

        let stream = match mic_config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &mic_config.clone().into(),
                move |data: &[f32], _| {
                    let mut buf = samples_clone.lock().unwrap();
                    if channels == 1 {
                        buf.extend_from_slice(data);
                    } else {
                        for frame in data.chunks(channels) {
                            let mono = frame.iter().sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => {
                let samples_clone2 = samples.clone();
                device.build_input_stream(
                    &mic_config.clone().into(),
                    move |data: &[i16], _| {
                        let mut buf = samples_clone2.lock().unwrap();
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
                )?
            }
            cpal::SampleFormat::U16 => {
                let samples_clone2 = samples.clone();
                device.build_input_stream(
                    &mic_config.clone().into(),
                    move |data: &[u16], _| {
                        let mut buf = samples_clone2.lock().unwrap();
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
                )?
            }
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
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let recorded = samples.lock().unwrap();
        println!("Captured {} mono samples", recorded.len());

        let mut writer = hound::WavWriter::create("voice.wav", spec)?;
        for &sample in recorded.iter() {
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

        println!("Done.\n");
    }

    Ok(())
}
