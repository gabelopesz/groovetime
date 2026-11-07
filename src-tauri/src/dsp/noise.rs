//! Estimativa do ruido de fundo: minimo do RMS (sobe devagar) em trechos "silenciosos".

/// Silencio acumulado necessario antes de mostrar um valor.
pub const MIN_SILENCE_SECS: f32 = 2.0;
/// Velocidade com que a estimativa pode subir (dB/s) — descer e imediato.
pub const RISE_DB_PER_SEC: f32 = 3.0;

#[derive(Debug, Clone, Default)]
pub struct NoiseFloor {
    estimate_dbfs: Option<f32>,
    silence_secs: f32,
}

impl NoiseFloor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.estimate_dbfs = None;
        self.silence_secs = 0.0;
    }

    /// `rms_dbfs`: RMS atual; `quiet`: sem pitch e abaixo do gate; `dt_secs`: duracao do bloco.
    pub fn update(&mut self, rms_dbfs: f32, quiet: bool, dt_secs: f32) {
        if !quiet {
            // Um tom no meio zera a espera: so vale silencio continuo.
            self.silence_secs = 0.0;
            return;
        }
        self.silence_secs += dt_secs;
        self.estimate_dbfs = Some(match self.estimate_dbfs {
            None => rms_dbfs,
            Some(est) => rms_dbfs.min(est + RISE_DB_PER_SEC * dt_secs),
        });
    }

    /// Valor estimado, depois de `MIN_SILENCE_SECS` de silencio acumulado.
    pub fn value(&self) -> Option<f32> {
        if self.silence_secs >= MIN_SILENCE_SECS { self.estimate_dbfs } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{RmsMeter, rms::RMS_WINDOW_SECS};

    const RATE: u32 = 192_000;

    /// Ruido branco uniforme com RMS alvo em dBFS.
    fn noise(rms_db: f32, n: usize, seed: &mut u32) -> Vec<f32> {
        let amp = 10f32.powf(rms_db / 20.0) * 3f32.sqrt(); // RMS de uniforme[-a,a] = a/sqrt(3)
        (0..n)
            .map(|_| {
                *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((*seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0) * amp
            })
            .collect()
    }

    #[test]
    fn measures_white_noise_floor() {
        let (mut rms, mut nf, mut seed) = (RmsMeter::new(RATE, RMS_WINDOW_SECS), NoiseFloor::new(), 1u32);
        for b in noise(-70.0, RATE as usize * 4, &mut seed).chunks(384) {
            let level = rms.process(b);
            nf.update(level, true, b.len() as f32 / RATE as f32);
        }
        let v = nf.value().unwrap();
        assert!((v + 70.0).abs() < 2.0, "{v}");
    }

    #[test]
    fn none_until_enough_silence_and_resets_on_tone() {
        let mut nf = NoiseFloor::new();
        for _ in 0..100 {
            nf.update(-70.0, true, 0.01); // 1 s
        }
        assert!(nf.value().is_none());
        nf.update(-20.0, false, 0.01); // tom interrompe
        for _ in 0..150 {
            nf.update(-70.0, true, 0.01); // 1.5 s
        }
        assert!(nf.value().is_none());
        for _ in 0..100 {
            nf.update(-70.0, true, 0.01);
        }
        assert!(nf.value().is_some());
    }

    #[test]
    fn rises_slowly_and_falls_immediately() {
        let mut nf = NoiseFloor::new();
        nf.update(-70.0, true, 0.01);
        nf.update(-50.0, true, 0.01); // sobe no maximo 3 dB/s * 10 ms
        assert!(nf.estimate_dbfs.unwrap() < -69.9);
        nf.update(-80.0, true, 0.01);
        assert_eq!(nf.estimate_dbfs, Some(-80.0));
    }
}
