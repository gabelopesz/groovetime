//! Decimador: passa-baixa Butterworth de 6a ordem (3 biquads) + downsample inteiro.

use std::f64::consts::PI;

const SECTIONS: usize = 3;
/// Q de cada secao de um Butterworth de ordem 6.
const BUTTERWORTH6_Q: [f64; SECTIONS] = [0.517_638_09, std::f64::consts::FRAC_1_SQRT_2, 1.931_851_65];

#[derive(Debug, Clone, Copy, Default)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn lowpass(fs: f64, fc: f64, q: f64) -> Self {
        let w0 = 2.0 * PI * fc / fs;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos) / 2.0 / a0,
            b1: (1.0 - cos) / a0,
            b2: (1.0 - cos) / 2.0 / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

#[derive(Debug, Clone)]
pub struct Decimator {
    factor: usize,
    output_rate: u32,
    sections: [Biquad; SECTIONS],
    phase: usize,
}

impl Decimator {
    /// `target_rate`: taxa de saida desejada; o fator inteiro e `round(input/target)`.
    /// `cutoff_hz`: corte do passa-baixa (limitado a 0.4x a taxa de saida).
    pub fn new(input_rate: u32, target_rate: u32, cutoff_hz: f32) -> Self {
        let factor = ((input_rate as f64 / target_rate.max(1) as f64).round() as usize).max(1);
        let output_rate = input_rate / factor as u32;
        let fs = input_rate as f64;
        let fc = (cutoff_hz as f64).min(0.4 * output_rate as f64).max(1.0);
        let mut sections = [Biquad::default(); SECTIONS];
        for (s, q) in sections.iter_mut().zip(BUTTERWORTH6_Q) {
            *s = Biquad::lowpass(fs, fc, q);
        }
        Self { factor, output_rate, sections, phase: 0 }
    }

    pub fn factor(&self) -> usize {
        self.factor
    }

    /// Taxa de saida efetiva (Hz).
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }

    pub fn reset(&mut self) {
        for s in &mut self.sections {
            s.z1 = 0.0;
            s.z2 = 0.0;
        }
        self.phase = 0;
    }

    /// Filtra e decima `input`, entregando cada amostra de saida a `sink`. Sem alocacao.
    pub fn process(&mut self, input: &[f32], mut sink: impl FnMut(f32)) {
        for &x in input {
            if self.factor == 1 {
                sink(x);
                continue;
            }
            let mut v = x as f64;
            for s in &mut self.sections {
                v = s.tick(v);
            }
            if self.phase == 0 {
                sink(v as f32);
            }
            self.phase += 1;
            if self.phase == self.factor {
                self.phase = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, fs: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / fs as f32).sin())
            .collect()
    }

    fn rms_after_settle(d: &mut Decimator, x: &[f32]) -> f32 {
        let mut out = Vec::new();
        d.process(x, |s| out.push(s));
        let tail = &out[out.len() / 2..];
        (tail.iter().map(|v| v * v).sum::<f32>() / tail.len() as f32).sqrt()
    }

    #[test]
    fn factors() {
        assert_eq!(Decimator::new(192_000, 8_000, 1_500.0).factor(), 24);
        assert_eq!(Decimator::new(48_000, 8_000, 1_500.0).factor(), 6);
        assert_eq!(Decimator::new(8_000, 8_000, 1_500.0).factor(), 1);
        assert_eq!(Decimator::new(44_100, 8_000, 1_500.0).output_rate(), 7_350);
    }

    #[test]
    fn passes_bass_with_unity_gain() {
        let mut d = Decimator::new(192_000, 8_000, 1_500.0);
        let r = rms_after_settle(&mut d, &tone(100.0, 192_000, 192_000));
        assert!((r - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.02, "rms={r}");
    }

    #[test]
    fn rejects_alias_tone() {
        // 9 kHz seria dobrado para 1 kHz na saida de 8 kHz.
        let mut d = Decimator::new(48_000, 8_000, 1_500.0);
        let r = rms_after_settle(&mut d, &tone(9_000.0, 48_000, 48_000));
        assert!(r < 0.005, "rms={r}");
    }

    #[test]
    fn output_length() {
        let mut d = Decimator::new(192_000, 8_000, 1_500.0);
        let mut n = 0;
        d.process(&vec![0.0; 192_000], |_| n += 1);
        assert_eq!(n, 8_000);
    }
}
