use fft_analizer::FrequencySpectrum;
use ringbuf::{storage::Heap, traits::*, wrap::caching::Caching, SharedRb};
use std::{sync::Arc, time::Duration};

pub struct InputModel<T: Producer<Item = f32>> {
    pub producer: T,
}

/// This monster is derived from the (HeapRb::<f32>).split() return type
pub type AudioProducerF32 = InputModel<Caching<Arc<SharedRb<Heap<f32>>>, true, false>>;
/// This monster is derived from the (HeapRb::<f32>).split() return type
pub type AudioConsumerF32<const IB_LEN: usize, const FB_LEN: usize, const DELTA: usize> =
    FftConsumer<IB_LEN, FB_LEN, DELTA, Caching<Arc<SharedRb<Heap<f32>>>, false, true>>;

pub struct FftConsumer<
    const IB_LEN: usize,
    const FB_LEN: usize,
    const DELTA: usize,
    T: Consumer<Item = f32>,
> {
    /// Consumer to read from shared buffer
    consumer: T,
    /// Input samples
    samples: [f32; IB_LEN],
    /// Processed frequencies
    pub frequencies: [f32; FB_LEN],
    /// Smoothed frequencies
    pub smoothed: [f32; FB_LEN],
    /// Read index
    index: usize,
    fs: FrequencySpectrum,
}

impl<const IB_LEN: usize, const FB_LEN: usize, const DELTA: usize, T: Consumer<Item = f32>>
    FftConsumer<IB_LEN, FB_LEN, DELTA, T>
{
    pub fn new(consumer: T, channels: u16) -> Self {
        FftConsumer {
            consumer,
            samples: [0.0; IB_LEN],
            frequencies: [0.0; FB_LEN],
            smoothed: [0.0; FB_LEN],
            index: 0,
            fs: FrequencySpectrum::new(IB_LEN, channels),
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
        let ff = self.fs.frequency_spectrum(&mut self.samples);
        for (i, f) in ff.iter().enumerate() {
            self.frequencies[i] = *f;
        }
        let m = (milis.as_nanos() / 1_000_000) as f64;
        for i in 0..ff.len() {
            self.smoothed[i] +=
                (ff[i] - self.smoothed[i]) as f32 * (m / 1000.0) as f32 * DELTA as f32;
        }

        self.index = 0;
    }

    // Updates the frequencies buffer by reading from input buffer and writing to frequencies array
    pub fn update(&mut self, milis: Duration) {
        self.read_samples();
        self.process_samples(milis);
    }
}
