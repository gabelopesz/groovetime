//! Contador de eventos de clip (cruzamento de -3 dBFS), nao de amostras.

/// Limiar que conta um evento.
pub const CLIP_THRESHOLD_DBFS: f32 = -3.0;
/// Abaixo disto (por `REARM_QUIET_SECS` seguidos) o contador rearma.
pub const REARM_DBFS: f32 = -6.0;
/// Tempo continuo abaixo de `REARM_DBFS` para rearmar (> periodo de B0 ~ 32 ms,
/// senao os cruzamentos por zero de uma nota grave contariam varios eventos).
pub const REARM_QUIET_SECS: f32 = 0.1;

#[derive(Debug, Clone)]
pub struct ClipCounter {
    count: u32,
    armed: bool,
    quiet_secs: f32,
}

impl ClipCounter {
    pub fn new() -> Self {
        Self { count: 0, armed: true, quiet_secs: 0.0 }
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    /// Zera a contagem (e rearma).
    pub fn reset(&mut self) {
        self.count = 0;
        self.armed = true;
        self.quiet_secs = 0.0;
    }

    /// `block_peak_dbfs`: pico do bloco; `dt_secs`: duracao do bloco.
    pub fn update(&mut self, block_peak_dbfs: f32, dt_secs: f32) {
        if block_peak_dbfs >= CLIP_THRESHOLD_DBFS {
            if self.armed {
                self.count = self.count.saturating_add(1);
                self.armed = false;
            }
            self.quiet_secs = 0.0;
        } else if block_peak_dbfs >= REARM_DBFS {
            self.quiet_secs = 0.0;
        } else if !self.armed {
            self.quiet_secs += dt_secs;
            if self.quiet_secs >= REARM_QUIET_SECS {
                self.armed = true;
                self.quiet_secs = 0.0;
            }
        }
    }
}

impl Default for ClipCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::peak::linear_to_dbfs;
    use std::f32::consts::PI;

    const RATE: f32 = 192_000.0;

    /// Alimenta o contador com blocos de 384 amostras como o engine.
    fn feed(c: &mut ClipCounter, x: &[f32]) {
        for b in x.chunks(384) {
            let peak = b.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            c.update(linear_to_dbfs(peak), b.len() as f32 / RATE);
        }
    }

    fn burst(amp: f32, secs: f32) -> Vec<f32> {
        (0..(RATE * secs) as usize).map(|i| amp * (2.0 * PI * 41.2 * i as f32 / RATE).sin()).collect()
    }

    #[test]
    fn one_event_per_burst_not_per_sample_or_cycle() {
        let mut c = ClipCounter::new();
        feed(&mut c, &burst(0.95, 0.5)); // ~20 periodos acima de -3 dB
        assert_eq!(c.count(), 1);
        feed(&mut c, &vec![0.0; (RATE * 0.5) as usize]);
        feed(&mut c, &burst(0.95, 0.5));
        assert_eq!(c.count(), 2);
    }

    #[test]
    fn below_threshold_never_counts() {
        let mut c = ClipCounter::new();
        feed(&mut c, &burst(0.5, 1.0)); // -6 dBFS
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn decaying_note_between_minus_6_and_minus_3_does_not_rearm() {
        let mut c = ClipCounter::new();
        feed(&mut c, &burst(0.95, 0.2));
        // Cai para ~ -4.5 dBFS e fica ali por 1 s (histerese).
        feed(&mut c, &burst(0.6, 1.0));
        feed(&mut c, &burst(0.95, 0.2));
        assert_eq!(c.count(), 1);
    }

    #[test]
    fn reset_clears_count() {
        let mut c = ClipCounter::new();
        feed(&mut c, &burst(0.95, 0.3));
        assert_eq!(c.count(), 1);
        c.reset();
        assert_eq!(c.count(), 0);
    }
}
