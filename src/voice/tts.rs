/*
git submodule update --init

wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/libritts_r/medium/en_US-libritts_r-medium.onnx.json
cargo run --example usage en_US-libritts_r-medium.onnx.json 80
*/

use piper_rs::Piper;
use rodio::buffer::SamplesBuffer;
use std::path::Path;

pub fn generate_voice() {
    let config_path = "./src/voice/model.onnx.json";
    let onnx_path = "./src/voice/model-amy.onnx";
    let speaker_id: Option<i64> = Some(80);

    let mut piper = Piper::new(Path::new(onnx_path), Path::new(config_path)).unwrap();

    let text = "Hello! I'm playing audio from memory directly with piper-rs.";
    let (samples, sample_rate) = piper
        .create(text, false, speaker_id, None, None, None)
        .unwrap();

    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();
    sink.append(SamplesBuffer::new(1, sample_rate, samples));
    sink.sleep_until_end();
}
