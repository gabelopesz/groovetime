//! RMS estilo VU: potencia media com integracao exponencial (~300 ms).

use super::peak::FLOOR_DBFS;

/// Constante de tempo da integracao (VU classico ~300 ms).
pub const RMS_WINDOW_SECS: f32 = 0.3;

#[derive(Debug, Clone)]
pub struct RmsMeter {
    sample_rate: f32,
    window_secs: f32,
    power: f32,
    /// O primeiro bloco inicializa a potencia (evita a rampa a partir de zero).
    primed: bool,
}

impl RmsMeter {
    pub fn new(sample_rate: u32, window_secs: f32) -> Self {
        Self { sample_rate: sample_rate.max(1) as f32, window_secs: window_secs.max(1e-3), power: 0.0, primed: false }
    }

    pub fn reset(&mut self) {
        self.power = 0.0;
        self.primed = false;
    }

    /// Processa um bloco (sem alocacao) e devolve o RMS atual em dBFS.
    pub fn process(&mut self, block: &[f32]) -> f32 {
        if !block.is_empty() {
            let mut sum = 0.0f32;
            for &s in block {
                if s.is_finite() {
                    sum += s * s;
                }
            }
            let mean_square = sum / block.len() as f32;
            if self.primed {
                let dt = block.len() as f32 / self.sample_rate;
                let alpha = 1.0 - (-dt / self.window_secs).exp();
                self.power += alpha * (mean_square - self.power);
            } else {
                self.power = mean_square;
                self.primed = true;
            }
        }
        self.dbfs()
    }

    pub fn dbfs(&self) -> f32 {
        if self.power <= 0.0 {
            return FLOOR_DBFS;
        }
        (10.0 * self.power.log10()).max(FLOOR_DBFS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn run(m: &mut RmsMeter, x: &[f32]) -> f32 {
        let mut last = FLOOR_DBFS;
        for c in x.chunks(384) {
            last = m.process(c);
        }
        last
    }

    #[test]
    fn full_scale_sine_is_minus_3_dbfs() {
        let mut m = RmsMeter::new(192_000, RMS_WINDOW_SECS);
        let x: Vec<f32> = (0..192_000 * 2).map(|i| (2.0 * PI * 1000.0 * i as f32 / 192_000.0).sin()).collect();
        let r = run(&mut m, &x);
        assert!((r + 3.01).abs() < 0.1, "{r}");
    }

    #[test]
    fn silence_is_floor_and_decays_smoothly() {
        let mut m = RmsMeter::new(48_000, RMS_WINDOW_SECS);
        assert_eq!(run(&mut m, &[0.0; 4800]), FLOOR_DBFS);
        run(&mut m, &vec![0.5; 48_000]);
        let after = run(&mut m, &vec![0.0; 4_800]); // 100 ms depois
        assert!(after < -6.0 && after > -30.0, "{after}");
    }

    #[test]
    fn ignores_non_finite() {
        let mut m = RmsMeter::new(48_000, RMS_WINDOW_SECS);
        let r = m.process(&[f32::NAN, f32::INFINITY, 0.1, 0.1]);
        assert!(r.is_finite());
    }
}
