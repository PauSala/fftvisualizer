use std::{sync::Arc, time::Duration};

use ringbuf::{storage::Heap, traits::Consumer, wrap::caching::Caching, SharedRb};

pub struct Bandpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Bandpass {
    fn new(f0: f32, q: f32, fs: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * f0 / fs;
        let alpha = w0.sin() / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x + self.z2 - self.a1 * y;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

pub struct FilterBankConsumer<
    const IB_LEN: usize,
    const FB_LEN: usize,
    const DELTA: usize,
    T: Consumer<Item = f32>,
> {
    consumer: T,
    pub samples: [f32; IB_LEN],
    pub frequencies: [f32; FB_LEN], // now: filter energies
    pub smoothed: [f32; FB_LEN],
    pub compressed: [f32; 12],
    index: usize,
    filters: Vec<Bandpass>, // filter bank
}

pub type AudioConsumerFilterBankF32<const IB_LEN: usize, const FB_LEN: usize, const DELTA: usize> =
    FilterBankConsumer<IB_LEN, FB_LEN, DELTA, Caching<Arc<SharedRb<Heap<f32>>>, false, true>>;

impl<const IB_LEN: usize, const FB_LEN: usize, const DELTA: usize, T: Consumer<Item = f32>>
    FilterBankConsumer<IB_LEN, FB_LEN, DELTA, T>
{
    pub fn new(consumer: T, sample_rate: f32, f_min: f32, f_max: f32) -> Self {
        // log-spaced frequencies
        let mut filters = Vec::new();
        let q = 200.0; // quality factor (adjust for bandwidth)
        let mut f = f_min;

        while f <= f_max && filters.len() < FB_LEN {
            filters.push(Bandpass::new(f, q, sample_rate));
            f *= 2f32.powf(1.0 / 12.0); // semitone steps
        }

        FilterBankConsumer {
            consumer,
            samples: [0.0; IB_LEN],
            frequencies: [0.0; FB_LEN],
            smoothed: [0.0; FB_LEN],
            compressed: [0.0; 12],
            index: 0,
            filters,
        }
    }

    fn read_samples(&mut self) {
        while let Some(sample) = self.consumer.try_pop() {
            self.samples[self.index] = sample;
            self.index += 1;
            if self.index == IB_LEN {
                break;
            }
        }
    }

    fn process_samples(&mut self, milis: Duration) {
        if self.index < IB_LEN - 1 {
            return;
        }

        for i in 0..12 {
            self.compressed[i] = 0.0;
        }

        for (i, filter) in self.filters.iter_mut().enumerate() {
            let mut energy = 0.0;
            for &s in self.samples.iter() {
                let y = filter.process(s);
                energy += y * y;
            }
            self.frequencies[i] = (energy / IB_LEN as f32).sqrt(); // RMS energy
            let note_index = i % 12;
            self.compressed[note_index] += self.frequencies[i];
        }

        let m = (milis.as_nanos() / 1_000_000) as f64;
        for i in 0..self.frequencies.len() {
            self.smoothed[i] +=
                (self.frequencies[i] - self.smoothed[i]) * (m / 1000.0) as f32 * DELTA as f32;
        }
        // --- 3. Statistical Analysis and Gating (NEW LOGIC) ---

        // a. Calculate Mean
        let sum: f32 = self.smoothed.iter().sum();
        let mean = sum / self.smoothed.len() as f32;

        // b. Calculate Standard Deviation (σ)
        let mut variance_sum = 0.0;
        for &magnitude in self.smoothed.iter() {
            variance_sum += (magnitude - mean).powi(2);
        }
        let variance = variance_sum / self.smoothed.len() as f32;
        let std_dev = variance.sqrt();

        // c. Calculate Median (Requires sorting a copy)
        // Note: Rust's partial_cmp is needed for f32 comparison.
        let mut sorted_magnitudes: Vec<f32> = self.smoothed.to_vec();
        sorted_magnitudes.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mid = sorted_magnitudes.len() / 2;
        // For 12 bins (even number), the median is the average of the two middle elements
        let median = (sorted_magnitudes[mid - 1] + sorted_magnitudes[mid]) / 2.0;

        // d. Define and Apply Adaptive Threshold (Gating)
        // Tune the THRESHOLD_FACTOR (k) to control filtering strictness:
        // k=0.0 means threshold = median (strictest).
        // k=1.0 means threshold = median + 1 * std_dev (more lenient).
        const THRESHOLD_FACTOR: f32 = 1.0;
        let threshold = median + THRESHOLD_FACTOR * std_dev;

        // Ensure we don't zero out everything if sound is very quiet
        let floor_threshold = threshold.max(0.001); // A small floor to prevent underflow

        // Define the potentiation range:
        let min_factor = 0.05; // Factor at i=0 (Threshold is reduced by 50% -> Looser gate)
        let max_factor = 3.0; // Factor at i=43 (Threshold is increased by 50% -> Stricter gate)

        let range = max_factor - min_factor;

        // Apply the gate: zero out bins below the final_threshold
        for i in 0..self.smoothed.len() {
            // i runs from 0 to 43 (44 semitones)

            let base_threshold = floor_threshold;

            // Normalize index i from [0, 43] to [0.0, 1.0]
            let normalized_i = i as f32 / (44.0 - 1.0);

            // Calculate the Potentiation Factor (starts low, ends high)
            // The factor starts at min_factor (0.5) and increases linearly to max_factor (1.5).
            let weighting_factor = min_factor + (normalized_i * range);

            // Apply the correction: The threshold is scaled by the factor
            let final_weighted_threshold = base_threshold * weighting_factor;

            // Apply the gate:
            if self.smoothed[i] < final_weighted_threshold {
                self.smoothed[i] = 0.0;
            }
        }

        self.index = 0;
    }

    pub fn update(&mut self, milis: Duration) {
        self.read_samples();
        self.process_samples(milis);
    }
}
