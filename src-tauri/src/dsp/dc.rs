//! DC offset: media movel (EMA, ~1 s) do sinal na taxa cheia.

/// Constante de tempo da media.
pub const DC_WINDOW_SECS: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct DcOffset {
    sample_rate: f32,
    window_secs: f32,
    mean: f32,
}

impl DcOffset {
    pub fn new(sample_rate: u32, window_secs: f32) -> Self {
        Self { sample_rate: sample_rate.max(1) as f32, window_secs: window_secs.max(1e-3), mean: 0.0 }
    }

    pub fn reset(&mut self) {
        self.mean = 0.0;
    }

    /// Atualiza com um bloco (sem alocacao).
    pub fn process(&mut self, block: &[f32]) {
        if block.is_empty() {
            return;
        }
        let mut sum = 0.0f32;
        for &s in block {
            if s.is_finite() {
                sum += s;
            }
        }
        let dt = block.len() as f32 / self.sample_rate;
        let alpha = 1.0 - (-dt / self.window_secs).exp();
        self.mean += alpha * (sum / block.len() as f32 - self.mean);
    }

    /// Offset como fracao de full scale (-1..1).
    pub fn fraction(&self) -> f32 {
        self.mean
    }

    /// Offset em % de full scale.
    pub fn percent(&self) -> f32 {
        self.mean * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn measures_two_percent_dc_under_a_bass_tone() {
        let mut dc = DcOffset::new(192_000, DC_WINDOW_SECS);
        let x: Vec<f32> = (0..192_000 * 4)
            .map(|i| 0.5 * (2.0 * PI * 55.0 * i as f32 / 192_000.0).sin() + 0.02)
            .collect();
        for b in x.chunks(384) {
            dc.process(b);
        }
        assert!((dc.percent() - 2.0).abs() < 0.3, "{}", dc.percent());
    }

    #[test]
    fn zero_for_symmetric_signal() {
        let mut dc = DcOffset::new(48_000, DC_WINDOW_SECS);
        for _ in 0..500 {
            dc.process(&[0.3, -0.3, 0.3, -0.3]);
        }
        assert!(dc.percent().abs() < 0.01);
    }
}
