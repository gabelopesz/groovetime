//! Detector de pitch YIN (de Cheveigne & Kawahara, 2002), otimizado para graves.
//!
//! Todos os buffers sao alocados em `new`; `detect` nao aloca.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchEstimate {
    pub freq_hz: f32,
    /// `1 - d'(tau)`: 1.0 = perfeitamente periodico.
    pub clarity: f32,
}

/// Limiar absoluto sobre a CMND.
pub const DEFAULT_THRESHOLD: f32 = 0.15;
/// Gate de nivel padrao (RMS do frame, dBFS).
pub const DEFAULT_GATE_DBFS: f32 = -50.0;

#[derive(Debug, Clone)]
pub struct Yin {
    sample_rate: f32,
    window_size: usize,
    /// Comprimento de integracao `W = window_size - tau_max`.
    integration: usize,
    tau_min: usize,
    tau_max: usize,
    threshold: f32,
    gate_linear: f32,
    /// CMND, indices `0..=tau_max`.
    cmnd: Vec<f32>,
}

impl Yin {
    /// `window_size` precisa ser maior que `tau_max = sample_rate / min_freq`
    /// (idealmente >= 2.5x para integrar varios ciclos). Valores invalidos
    /// sao ajustados para o menor valor utilizavel.
    pub fn new(sample_rate: u32, window_size: usize, min_freq: f32, max_freq: f32) -> Self {
        let fs = sample_rate as f32;
        let tau_max = ((fs / min_freq.max(1.0)).ceil() as usize + 1).max(4);
        let tau_min = ((fs / max_freq.max(min_freq + 1.0)).floor() as usize).max(2);
        let window_size = window_size.max(tau_max + 2);
        Self {
            sample_rate: fs,
            window_size,
            integration: window_size - tau_max,
            tau_min: tau_min.min(tau_max - 1),
            tau_max,
            threshold: DEFAULT_THRESHOLD,
            gate_linear: db_to_linear(DEFAULT_GATE_DBFS),
            cmnd: vec![0.0; tau_max + 1],
        }
    }

    /// Tamanho de frame exigido por `detect`.
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.01, 0.5);
    }

    /// Gate de nivel: frames com RMS abaixo disso retornam `None`.
    pub fn set_gate_dbfs(&mut self, gate_dbfs: f32) {
        self.gate_linear = db_to_linear(gate_dbfs);
    }

    /// Detecta o pitch do frame (usa as primeiras `window_size` amostras).
    /// `None` se o frame for curto, abaixo do gate ou nao periodico.
    pub fn detect(&mut self, frame: &[f32]) -> Option<PitchEstimate> {
        if frame.len() < self.window_size {
            return None;
        }
        let frame = &frame[..self.window_size];

        let energy: f32 = frame.iter().map(|v| v * v).sum();
        let rms = (energy / self.window_size as f32).sqrt();
        if !rms.is_finite() || rms < self.gate_linear {
            return None;
        }

        self.difference_and_cmnd(frame);
        let tau = self.absolute_threshold()?;
        let (tau_refined, cmnd_at) = self.parabolic(tau);
        let freq_hz = self.sample_rate / tau_refined;
        if !freq_hz.is_finite() {
            return None;
        }
        Some(PitchEstimate { freq_hz, clarity: (1.0 - cmnd_at).clamp(0.0, 1.0) })
    }

    /// Passos 1-3: funcao diferenca e media cumulativa normalizada.
    fn difference_and_cmnd(&mut self, frame: &[f32]) {
        self.cmnd[0] = 1.0;
        let head = &frame[..self.integration];
        let mut running = 0.0f32;
        for tau in 1..=self.tau_max {
            let lagged = &frame[tau..tau + self.integration];
            let mut d = 0.0f32;
            for (a, b) in head.iter().zip(lagged) {
                let diff = a - b;
                d += diff * diff;
            }
            running += d;
            self.cmnd[tau] = if running > 0.0 { d * tau as f32 / running } else { 1.0 };
        }
    }

    /// Passo 4: primeiro vale abaixo do limiar (descendo ate o minimo local).
    fn absolute_threshold(&self) -> Option<usize> {
        let mut tau = self.tau_min;
        while tau < self.tau_max {
            if self.cmnd[tau] < self.threshold {
                while tau + 1 < self.tau_max && self.cmnd[tau + 1] < self.cmnd[tau] {
                    tau += 1;
                }
                return Some(tau);
            }
            tau += 1;
        }
        None
    }

    /// Passo 5: interpolacao parabolica ao redor de `tau`.
    fn parabolic(&self, tau: usize) -> (f32, f32) {
        if tau == 0 || tau >= self.tau_max {
            return (tau as f32, self.cmnd[tau]);
        }
        let (s0, s1, s2) = (self.cmnd[tau - 1], self.cmnd[tau], self.cmnd[tau + 1]);
        let denom = s0 - 2.0 * s1 + s2;
        if denom.abs() < f32::EPSILON {
            return (tau as f32, s1);
        }
        let shift = (0.5 * (s0 - s2) / denom).clamp(-1.0, 1.0);
        (tau as f32 + shift, s1 - 0.25 * (s0 - s2) * shift)
    }
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{Decimator, MAX_PITCH_HZ, MIN_PITCH_HZ, PITCH_SAMPLE_RATE, YIN_WINDOW};
    use std::f32::consts::PI;

    const STRINGS: [f32; 6] = [30.87, 41.20, 55.00, 73.42, 98.00, 130.81];

    fn cents(measured: f32, expected: f32) -> f32 {
        1200.0 * (measured / expected).log2().abs()
    }

    fn new_yin() -> Yin {
        Yin::new(PITCH_SAMPLE_RATE, YIN_WINDOW, MIN_PITCH_HZ, MAX_PITCH_HZ)
    }

    /// Soma de harmonicos com amplitudes relativas.
    fn harmonic(f0: f32, fs: u32, n: usize, amps: &[f32]) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let t = i as f32 / fs as f32;
                amps.iter()
                    .enumerate()
                    .map(|(k, a)| a * (2.0 * PI * f0 * (k + 1) as f32 * t + 0.3 * k as f32).sin())
                    .sum::<f32>()
                    * 0.3
            })
            .collect()
    }

    #[test]
    fn clean_sine_under_one_cent() {
        let mut yin = new_yin();
        for f in STRINGS {
            let x = harmonic(f, PITCH_SAMPLE_RATE, YIN_WINDOW, &[1.0]);
            let est = yin.detect(&x).expect("pitch");
            assert!(cents(est.freq_hz, f) < 1.0, "{f}: {} Hz", est.freq_hz);
        }
    }

    #[test]
    fn sawtooth_like_no_octave_error() {
        let mut yin = new_yin();
        let amps: Vec<f32> = (1..=8).map(|k| 1.0 / k as f32).collect();
        for f in STRINGS {
            let x = harmonic(f, PITCH_SAMPLE_RATE, YIN_WINDOW, &amps);
            let est = yin.detect(&x).expect("pitch");
            assert!(cents(est.freq_hz, f) < 1.0, "{f}: {} Hz", est.freq_hz);
        }
    }

    #[test]
    fn attenuated_fundamental_under_three_cents() {
        let mut yin = new_yin();
        let amps = [0.3, 1.0, 0.7, 0.5];
        for f in STRINGS {
            let x = harmonic(f, PITCH_SAMPLE_RATE, YIN_WINDOW, &amps);
            let est = yin.detect(&x).expect("pitch");
            assert!(cents(est.freq_hz, f) < 3.0, "{f}: {} Hz", est.freq_hz);
        }
    }

    #[test]
    fn full_chain_192k_to_pitch() {
        let mut dec = Decimator::new(192_000, PITCH_SAMPLE_RATE, 1_500.0);
        let mut yin = new_yin();
        let amps: Vec<f32> = (1..=6).map(|k| 1.0 / k as f32).collect();
        for f in [41.20, 98.00] {
            dec.reset();
            let x = harmonic(f, 192_000, 192_000 / 2, &amps);
            let mut y = Vec::new();
            dec.process(&x, |s| y.push(s));
            let est = yin.detect(&y[y.len() - YIN_WINDOW..]).expect("pitch");
            assert!(cents(est.freq_hz, f) < 2.0, "{f}: {} Hz", est.freq_hz);
        }
    }

    #[test]
    fn silence_and_low_noise_are_none() {
        let mut yin = new_yin();
        assert!(yin.detect(&vec![0.0; YIN_WINDOW]).is_none());
        // Ruido pseudo-aleatorio abaixo do gate (~ -70 dBFS).
        let mut seed = 12345u32;
        let noise: Vec<f32> = (0..YIN_WINDOW)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * 0.001
            })
            .collect();
        assert!(yin.detect(&noise).is_none());
    }

    #[test]
    fn short_frame_is_none() {
        let mut yin = new_yin();
        assert!(yin.detect(&[0.5; 100]).is_none());
    }

    #[test]
    fn gate_is_adjustable() {
        let mut yin = new_yin();
        let x: Vec<f32> =
            harmonic(55.0, PITCH_SAMPLE_RATE, YIN_WINDOW, &[1.0]).iter().map(|v| v * 0.01).collect();
        // Amplitude 0.003 -> RMS ~ -53 dBFS: abaixo do gate padrao (-50).
        assert!(yin.detect(&x).is_none());
        yin.set_gate_dbfs(-80.0);
        assert!(yin.detect(&x).is_some());
    }

    #[test]
    fn detect_fits_time_budget() {
        let mut yin = new_yin();
        let x = harmonic(30.87, PITCH_SAMPLE_RATE, YIN_WINDOW, &[1.0, 0.5, 0.3]);
        let start = std::time::Instant::now();
        for _ in 0..20 {
            let _ = yin.detect(&x);
        }
        let avg = start.elapsed() / 20;
        assert!(avg < std::time::Duration::from_millis(8), "avg={avg:?}");
    }
}
