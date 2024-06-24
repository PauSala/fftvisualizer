pub struct HannWindow {
    window: Vec<f32>,
}
impl HannWindow {
    pub fn new(n: usize) -> Self {
        let mut window = Vec::with_capacity(n);
        for i in 0..n {
            let value =
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos());
            window.push(value);
        }
        HannWindow { window }
    }

    pub fn apply(&self, samples: &mut [f32]) {
        if samples.len() != self.window.len() {
            panic!(
                "samples len {} is different than window len {}",
                samples.len(),
                self.window.len()
            )
        }
        for (i, sample) in samples.iter_mut().enumerate() {
            *sample *= self.window[i];
        }
    }
}
