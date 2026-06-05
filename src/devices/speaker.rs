use rodio::{OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer};

/// Open the default audio output device.
/// The caller must keep `OutputStream` alive for the duration of playback.
pub fn open_output() -> (OutputStream, OutputStreamHandle) {
    OutputStream::try_default().expect("failed to open audio output")
}

/// Play raw mono f32 samples at the given sample rate, blocking until done.
pub fn play_samples(handle: &OutputStreamHandle, samples: &[f32], sample_rate: u32) {
    let sink = Sink::try_new(handle).expect("failed to create sink");
    let source = SamplesBuffer::new(1, sample_rate, samples.to_vec());
    sink.append(source);
    sink.sleep_until_end();
}
