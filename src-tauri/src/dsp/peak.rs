//! Medidor de pico em dBFS com hold e decay.

/// Limiar do alerta PEAK (dBFS).
pub const PEAK_ALERT_DBFS: f32 = -3.0;
/// Piso de silencio (dBFS).
pub const FLOOR_DBFS: f32 = -120.0;
/// Tempo que o hold segura o maior pico antes de cair.
pub const PEAK_HOLD_SECS: f32 = 1.5;
/// Velocidade de queda do hold depois do tempo de hold.
pub const PEAK_DECAY_DB_PER_SEC: f32 = 20.0;

/// Leitura do medidor apos processar um bloco.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakReading {
    /// Pico do bloco (dBFS).
    pub peak_dbfs: f32,
    /// Pico segurado, com decay (dBFS).
    pub hold_dbfs: f32,
    /// `peak_dbfs >= threshold` neste bloco.
    pub over_threshold: bool,
}

/// Converte amplitude linear (0..1) em dBFS, com piso.
pub fn linear_to_dbfs(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        return FLOOR_DBFS;
    }
    (20.0 * amplitude.log10()).max(FLOOR_DBFS)
}

#[derive(Debug, Clone)]
pub struct PeakMeter {
    sample_rate: f32,
    threshold_dbfs: f32,
    hold_secs: f32,
    decay_db_per_sec: f32,
    hold_dbfs: f32,
    since_peak_secs: f32,
}

impl PeakMeter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate.max(1) as f32,
            threshold_dbfs: PEAK_ALERT_DBFS,
            hold_secs: PEAK_HOLD_SECS,
            decay_db_per_sec: PEAK_DECAY_DB_PER_SEC,
            hold_dbfs: FLOOR_DBFS,
            since_peak_secs: 0.0,
        }
    }

    pub fn set_threshold_dbfs(&mut self, threshold: f32) {
        self.threshold_dbfs = threshold;
    }

    pub fn reset(&mut self) {
        self.hold_dbfs = FLOOR_DBFS;
        self.since_peak_secs = 0.0;
    }

    /// Processa um bloco (sem alocacao). Amostras nao finitas sao ignoradas.
    pub fn process(&mut self, block: &[f32]) -> PeakReading {
        let mut max = 0.0f32;
        for &s in block {
            let a = s.abs();
            if a > max && a.is_finite() {
                max = a;
            }
        }
        let peak_dbfs = linear_to_dbfs(max);
        let dt = block.len() as f32 / self.sample_rate;

        if peak_dbfs >= self.hold_dbfs {
            self.hold_dbfs = peak_dbfs;
            self.since_peak_secs = 0.0;
        } else {
            self.since_peak_secs += dt;
            if self.since_peak_secs > self.hold_secs {
                self.hold_dbfs = (self.hold_dbfs - self.decay_db_per_sec * dt).max(FLOOR_DBFS);
            }
        }

        PeakReading {
            peak_dbfs,
            hold_dbfs: self.hold_dbfs,
            over_threshold: peak_dbfs >= self.threshold_dbfs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Senoide de 1 kHz a 48 kHz cujas amostras atingem o pico exato.
    fn sine(amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn silence_is_floor() {
        let mut m = PeakMeter::new(48_000);
        let r = m.process(&[0.0; 480]);
        assert_eq!(r.peak_dbfs, FLOOR_DBFS);
        assert!(!r.over_threshold);
    }

    #[test]
    fn minus_3_dbfs_triggers_alert_minus_6_does_not() {
        let mut m = PeakMeter::new(48_000);
        assert!(m.process(&sine(0.708, 480)).over_threshold);
        let r = m.process(&sine(0.5, 480));
        assert!(!r.over_threshold);
        assert!((r.peak_dbfs + 6.02).abs() < 0.05);
    }

    #[test]
    fn hold_then_decay() {
        let mut m = PeakMeter::new(48_000);
        let high = m.process(&sine(0.5, 480)).hold_dbfs;
        // Durante o hold (1.0 s) o valor se mantem.
        for _ in 0..100 {
            m.process(&[0.0; 480]);
        }
        assert_eq!(m.process(&[0.0; 480]).hold_dbfs, high);
        // Depois de 1.5 s comeca a cair a ~20 dB/s.
        // Mais 1 s depois do hold: ~20 dB de queda.
        for _ in 0..150 {
            m.process(&[0.0; 480]);
        }
        let later = m.process(&[0.0; 480]).hold_dbfs;
        assert!((high - later - 20.0).abs() < 1.0, "later={later} high={high}");
    }

    #[test]
    fn ignores_non_finite() {
        let mut m = PeakMeter::new(48_000);
        let r = m.process(&[f32::NAN, f32::INFINITY, 0.1]);
        assert!((r.peak_dbfs + 20.0).abs() < 0.01);
    }
}
