pub mod hann_window;

use hann_window::HannWindow;
use rustfft::{num_complex::Complex, FftPlanner};

/// A struct for computing the frequency spectrum of audio samples using FFT.
pub struct FrequencySpectrum {
    hann_window: HannWindow,
    samples_mut: Vec<f32>,
    channels: u16,
}

impl FrequencySpectrum {
    pub fn new(samples_len: usize, channels: u16) -> Self {
        let len = samples_len / channels as usize;
        let hann_window = HannWindow::new(len);
        let samples_mut = vec![0.0; len];
        FrequencySpectrum {
            hann_window,
            samples_mut,
            channels,
        }
    }
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
    pub fn frequency_spectrum(&mut self, samples: &[f32]) -> Vec<f32> {
        self.mix_channels(samples);
        self.hann_window.apply(&mut self.samples_mut);
        let mut ff = self.fft();
        FrequencySpectrum::normalize(&mut ff);
        ff
    }

    /// Mixes audio samples across channels by averaging them.
    ///
    /// If `channels` is 1, assumes mono audio and directly uses `samples`.
    /// If `channels` is greater than 1, averages samples across channels.
    fn mix_channels(&mut self, samples: &[f32]) {
        let mut v: f32 = 0.0;

        if self.channels == 1 {
            // Mono audio: directly use the samples
            for (i, e) in samples.iter().enumerate() {
                self.samples_mut[i] = *e;
            }
        } else {
            // Multi-channel audio: average samples across channels
            let mut j = 0;
            for i in 0..samples.len() {
                v += samples[i];
                if (i + 1) % self.channels as usize == 0 {
                    self.samples_mut[j] = v / self.channels as f32;
                    j += 1;
                    v = 0.0;
                }
            }
        }
    }

    /// Computes fft over given samples.
    ///
    /// The second half and the DC are discarted since they are not relevant for audio processing.
    fn fft(&mut self) -> Vec<f32> {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(self.samples_mut.len());
        let mut spectrum: Vec<Complex<f32>> = self
            .samples_mut
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

    /// Normalizes between 0 and 1
    fn normalize(input: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }

        let min = *input
            .iter()
            .min_by(|a, b| {
                a.partial_cmp(b)
                    .expect(&format!("Can't compare this values: {} {}", a, b))
            })
            .expect("This vector shouln'd be empty");
        let max = *input
            .iter()
            .max_by(|a, b| {
                a.partial_cmp(b)
                    .expect(&format!("Can't compare this values: {} {}", a, b))
            })
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
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;

    fn sinus_wave() -> Vec<f32> {
        let mut res: Vec<f32> = vec![0.0; 360];
        for e in 0..res.len() {
            let sin = (e as f32 * PI / 180.0).sin() + 1.0;
            res[e] = sin;
        }
        res
    }

    #[test]
    fn should_fill_first_bin_to_one_for_sinus_waves() {
        let mut samples = sinus_wave();
        let mut fs = FrequencySpectrum::new(samples.len(), 1);
        let res = fs.frequency_spectrum(&mut samples);
        assert_eq!(res[0], 1.0);
    }

    #[test]
    fn mix_channels() {
        let samples = vec![1.0, 2.0, 2.0, 3.0];
        let mut fs = FrequencySpectrum::new(samples.len(), 2);
        fs.mix_channels(&samples);
        assert_eq!(vec![1.5, 2.5], fs.samples_mut);

        let samples = vec![1.0, 2.0, 3.0, 3.0, 4.0, 5.0];
        let mut fs = FrequencySpectrum::new(samples.len(), 3);
        fs.mix_channels(&samples);
        assert_eq!(vec![2.0, 4.0], fs.samples_mut);
    }

    #[test]
    fn normalize_all_values_should_be_in_zero_one_range() {
        let mut input = vec![0.0, 0.1, 0.1, 0.05];
        FrequencySpectrum::normalize(&mut input);
        assert_eq!(input, vec![0.0, 1.0, 1.0, 0.5]);
    }
    #[test]
    fn normalize_all_values_are_equal_and_greater_than_zero() {
        let mut input = vec![0.1, 0.1, 0.1, 0.1];
        FrequencySpectrum::normalize(&mut input);
        assert_eq!(input, vec![1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn normalize_all_values_are_equal_to_zero() {
        let mut input = vec![0.0, 0.0, 0.0, 0.0];
        FrequencySpectrum::normalize(&mut input);
        assert_eq!(input, vec![0.0, 0.0, 0.0, 0.0]);
        sinus_wave();
    }
}
