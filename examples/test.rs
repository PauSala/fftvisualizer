use fft_analizer::FrequencySpectrum;
use rand::Rng; // Add rand crate to your dependencies

fn generate_random_noise(length: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    let mut samples = Vec::with_capacity(length);

    for _ in 0..length {
        samples.push(rng.gen_range(-1.0..1.0));
    }

    samples
}

pub fn main() {
    let samples = generate_random_noise(20);
    let mut analizer = FrequencySpectrum::new(samples.len(), 1);
    let data = analizer.frequency_spectrum(&samples);
    println!("This is working! {:?} ", data);
}
