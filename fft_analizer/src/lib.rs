use rustfft::{num_complex::Complex, FftPlanner};

/// Computes the frequency spectrum of audio samples using FFT.
///
/// Takes a slice of audio samples and computes the FFT (Fast Fourier Transform)
/// to obtain the frequency domain representation.
///
/// If `channels` is 1, assumes mono audio and directly uses `samples`.
/// If `channels` is greater than 1, assumes interleaved stereo or multi-channel audio
/// and averages samples across channels before computing FFT.
///
/// Applies a Hann window to the samples before FFT to reduce spectral leakage.
/// Normalizes the FFT output to ensure consistent magnitude scaling.
///
/// # Arguments
///
/// * `samples` - A slice containing the audio samples.
/// * `channels` - Number of audio channels (1 for mono, 2 for stereo, etc.).
///
/// # Returns
///
/// A vector containing the magnitudes of the frequency bins from the FFT,
/// ignoring the DC component
///
pub fn frequency_spectrum(samples: &[f32], channels: u16) -> Vec<f32> {
    let mut mixed_samples = Vec::with_capacity(samples.len() / channels as usize);
    let mut v: f32 = 0.0;
    if channels == 1 {
        mixed_samples = samples.into();
    } else {
        for i in 0..samples.len() {
            if (i + 1) % channels as usize == 0 {
                mixed_samples.push(v / channels as f32);
                v = 0.0;
            }
        }
    }

    apply_hann_window(&mut mixed_samples);
    let mut frequencies = fft(&mixed_samples);
    normalize(&mut frequencies);
    frequencies
}

fn hann_window(n: usize) -> Vec<f32> {
    let mut window = Vec::with_capacity(n);
    for i in 0..n {
        let value = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos());
        window.push(value);
    }
    window
}

fn apply_hann_window(samples: &mut [f32]) {
    let n = samples.len();
    let window = hann_window(n);

    for (i, sample) in samples.iter_mut().enumerate() {
        *sample *= window[i];
    }
}

fn fft(samples: &[f32]) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(samples.len());
    let mut spectrum: Vec<Complex<f32>> = samples
        .iter()
        .map(|&sample| Complex::new(sample, 0.0))
        .collect();
    fft.process(&mut spectrum);
    let half = spectrum.len() / 2;
    spectrum
        .iter()
        .skip(1)
        .take(half)
        .map(|sample| sample.norm())
        .collect()
}

fn normalize(input: &mut Vec<f32>) {
    if input.is_empty() {
        return;
    }

    let min = *input
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .expect("This vector shouln'd be empty");
    let max = *input
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .expect("This vector shouln'd be empty");

    // Avoid division by zero
    if max == min {
        if min > 0.0 {
            for value in input.iter_mut() {
                *value = 1.0;
            }
        } else {
            for value in input.iter_mut() {
                *value = 0.0;
            }
        }
    } else {
        for value in input.iter_mut() {
            *value = (*value - min) / (max - min);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;

    fn sinus_wave() -> Vec<f32> {
        let mut res: Vec<f32> = vec![0.0; 360];
        for e in 0..res.len() {
            let sin = (e as f32 * PI / 180.0).sin() * 1000.0;
            res[e] = sin;
        }
        res
    }

    #[test]
    fn frequencies_should_fill_first_bin_to_one_for_sinus_waves() {
        let mut samples = sinus_wave();
        let res = frequency_spectrum(&mut samples, 1);
        assert_eq!(res[0], 1.0);
    }

    #[test]
    fn normalize_all_values_should_be_in_zero_one_range() {
        let mut input = vec![0.0, 0.1, 0.1, 0.05];
        normalize(&mut input);
        assert_eq!(input, vec![0.0, 1.0, 1.0, 0.5]);
    }
    #[test]
    fn normalize_all_values_are_equal_and_greater_than_zero() {
        let mut input = vec![0.1, 0.1, 0.1, 0.1];
        normalize(&mut input);
        assert_eq!(input, vec![1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn normalize_all_values_are_equal_and_are_zero() {
        let mut input = vec![0.0, 0.0, 0.0, 0.0];
        normalize(&mut input);
        assert_eq!(input, vec![0.0, 0.0, 0.0, 0.0]);
        sinus_wave();
    }
}
