use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

/// Create a sinc-based resampler for converting audio from `from_rate` to `to_rate`.
///
/// `chunk_size` is the number of input samples per processing call (e.g. 960
/// for 20 ms chunks at 48 kHz).
pub fn create_resampler(from_rate: u32, to_rate: u32, chunk_size: usize) -> SincFixedIn<f32> {
    let ratio = to_rate as f64 / from_rate as f64;

    let sinc_params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    SincFixedIn::<f32>::new(ratio, 2.0, sinc_params, chunk_size, 1)
        .expect("failed to create resampler")
}
