//! Pipeline de DSP (sem audio/Tauri): pico na taxa cheia, decimacao, YIN e
//! estabilizacao, agregados em `SignalFrame`.

use serde::Serialize;

use crate::audio::capture::{GAIN_ACTIVE, GAIN_PASSIVE};
use crate::dsp::note::DEFAULT_A4_HZ;
use crate::dsp::peak::FLOOR_DBFS;
use crate::dsp::{
    Decimator, MAX_PITCH_HZ, MIN_PITCH_HZ, PITCH_SAMPLE_RATE, PeakMeter, PitchSmoother,
    YIN_WINDOW, Yin, freq_to_note,
};

/// Por quanto tempo `clipped_recently` fica ligado apos cruzar o limiar.
pub const CLIP_HOLD_SECS: f32 = 1.5;
/// Corte do passa-baixa antes do YIN.
const DECIMATOR_CUTOFF_HZ: f32 = 1_500.0;
/// Leituras vazias seguidas (~16 ms cada) que ainda seguram a ultima nota.
const PITCH_HOLD_FRAMES: usize = 10;

/// Modo do baixo: Ativo aplica -12 dB digitais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BassMode {
    Active,
    Passive,
}

impl BassMode {
    pub fn from_gain(gain: f32) -> Self {
        if (gain - GAIN_ACTIVE).abs() < (gain - GAIN_PASSIVE).abs() {
            Self::Active
        } else {
            Self::Passive
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PitchInfo {
    pub freq_hz: f32,
    pub note: &'static str,
    pub octave: i32,
    pub cents: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalFrame {
    /// Maior pico do intervalo (dBFS), nunca a ultima amostra.
    pub peak_dbfs: f32,
    pub peak_hold_dbfs: f32,
    /// Houve pico >= limiar no intervalo.
    pub over_threshold: bool,
    /// Pico >= limiar nos ultimos `CLIP_HOLD_SECS`.
    pub clipped_recently: bool,
    pub pitch: Option<PitchInfo>,
    pub mode: BassMode,
    pub sample_rate: u32,
    pub dropped_samples: u64,
}

pub struct DspPipeline {
    sample_rate: u32,
    peak: PeakMeter,
    decimator: Decimator,
    yin: Yin,
    smoother: PitchSmoother,
    a4_hz: f32,
    /// Janela circular do sinal decimado.
    window: Vec<f32>,
    window_pos: usize,
    window_filled: usize,
    /// Janela linearizada para o YIN.
    frame_buf: Vec<f32>,
    interval_peak_dbfs: f32,
    interval_over: bool,
    hold_dbfs: f32,
    clip_remaining_secs: f32,
}

impl DspPipeline {
    pub fn new(sample_rate: u32) -> Self {
        let decimator = Decimator::new(sample_rate, PITCH_SAMPLE_RATE, DECIMATOR_CUTOFF_HZ);
        let yin = Yin::new(decimator.output_rate(), YIN_WINDOW, MIN_PITCH_HZ, MAX_PITCH_HZ);
        let window_size = yin.window_size();
        Self {
            sample_rate,
            peak: PeakMeter::new(sample_rate),
            decimator,
            yin,
            smoother: PitchSmoother::new(PITCH_HOLD_FRAMES),
            a4_hz: DEFAULT_A4_HZ,
            window: vec![0.0; window_size],
            window_pos: 0,
            window_filled: 0,
            frame_buf: vec![0.0; window_size],
            interval_peak_dbfs: FLOOR_DBFS,
            interval_over: false,
            hold_dbfs: FLOOR_DBFS,
            clip_remaining_secs: 0.0,
        }
    }

    pub fn set_a4_hz(&mut self, a4_hz: f32) {
        self.a4_hz = a4_hz;
    }

    pub fn set_gate_dbfs(&mut self, gate_dbfs: f32) {
        self.yin.set_gate_dbfs(gate_dbfs);
    }

    /// Alimenta o pipeline com um bloco de amostras (taxa cheia, ja com ganho).
    pub fn feed(&mut self, block: &[f32]) {
        if block.is_empty() {
            return;
        }
        let reading = self.peak.process(block);
        self.interval_peak_dbfs = self.interval_peak_dbfs.max(reading.peak_dbfs);
        self.interval_over |= reading.over_threshold;
        self.hold_dbfs = reading.hold_dbfs;
        if reading.over_threshold {
            self.clip_remaining_secs = CLIP_HOLD_SECS;
        } else {
            let dt = block.len() as f32 / self.sample_rate as f32;
            self.clip_remaining_secs = (self.clip_remaining_secs - dt).max(0.0);
        }

        let (window, pos, filled) = (&mut self.window, &mut self.window_pos, &mut self.window_filled);
        let len = window.len();
        self.decimator.process(block, |s| {
            window[*pos] = s;
            *pos = (*pos + 1) % len;
            *filled = (*filled + 1).min(len);
        });
    }

    /// Fecha o intervalo atual e produz o frame (reinicia o pico do intervalo).
    pub fn frame(&mut self, mode: BassMode, dropped_samples: u64) -> SignalFrame {
        let estimate = self.detect_pitch();
        let smoothed = self.smoother.update(estimate);
        let pitch = smoothed.and_then(|freq_hz| {
            freq_to_note(freq_hz, self.a4_hz).map(|n| PitchInfo {
                freq_hz,
                note: n.name,
                octave: n.octave,
                cents: n.cents,
            })
        });

        let frame = SignalFrame {
            peak_dbfs: self.interval_peak_dbfs,
            peak_hold_dbfs: self.hold_dbfs,
            over_threshold: self.interval_over,
            clipped_recently: self.clip_remaining_secs > 0.0,
            pitch,
            mode,
            sample_rate: self.sample_rate,
            dropped_samples,
        };
        self.interval_peak_dbfs = FLOOR_DBFS;
        self.interval_over = false;
        frame
    }

    fn detect_pitch(&mut self) -> Option<f32> {
        if self.window_filled < self.window.len() {
            return None;
        }
        // Linearizar a janela circular (mais antiga primeiro).
        let len = self.window.len();
        let (older, newer) = self.window.split_at(self.window_pos);
        self.frame_buf[..len - self.window_pos].copy_from_slice(newer);
        self.frame_buf[len - self.window_pos..].copy_from_slice(older);
        self.yin.detect(&self.frame_buf).map(|p| p.freq_hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const RATE: u32 = 192_000;

    fn bass_tone(freq: f32, amp: f32, secs: f32) -> Vec<f32> {
        (0..(RATE as f32 * secs) as usize)
            .map(|i| {
                let t = i as f32 / RATE as f32;
                amp * ((2.0 * PI * freq * t).sin() + 0.5 * (2.0 * PI * 2.0 * freq * t).sin()) / 1.5
            })
            .collect()
    }

    /// Alimenta em blocos de ~2 ms, como o loop real.
    fn feed_all(p: &mut DspPipeline, x: &[f32]) {
        for chunk in x.chunks(384) {
            p.feed(chunk);
        }
    }

    #[test]
    fn detects_a1_with_note_and_cents() {
        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &bass_tone(55.0, 0.5, 0.6));
        let f = p.frame(BassMode::Passive, 0);
        let pitch = f.pitch.expect("pitch");
        assert_eq!((pitch.note, pitch.octave), ("A", 1));
        assert!(pitch.cents.abs() < 2.0, "{}", pitch.cents);
    }

    #[test]
    fn silence_has_no_pitch_and_floor_peak() {
        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &vec![0.0; RATE as usize / 2]);
        let f = p.frame(BassMode::Passive, 0);
        assert!(f.pitch.is_none());
        assert_eq!(f.peak_dbfs, FLOOR_DBFS);
        assert!(!f.over_threshold && !f.clipped_recently);
    }

    #[test]
    fn interval_peak_is_the_max_not_the_last() {
        let mut p = DspPipeline::new(RATE);
        let mut x = bass_tone(55.0, 0.9, 0.01); // pico real ~ -2.2 dBFS
        x.extend(vec![0.0; 5_000]);
        feed_all(&mut p, &x);
        let f = p.frame(BassMode::Passive, 0);
        assert!(f.peak_dbfs > -3.0, "{}", f.peak_dbfs);
        assert!(f.over_threshold && f.clipped_recently);
        // O intervalo seguinte recomeca do piso, mas o LED segue aceso (hold).
        feed_all(&mut p, &vec![0.0; 384]);
        let f2 = p.frame(BassMode::Passive, 0);
        assert_eq!(f2.peak_dbfs, FLOOR_DBFS);
        assert!(!f2.over_threshold && f2.clipped_recently);
    }

    #[test]
    fn clipped_recently_expires() {
        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &bass_tone(55.0, 0.9, 0.01));
        feed_all(&mut p, &vec![0.0; (RATE as f32 * 1.6) as usize]);
        assert!(!p.frame(BassMode::Passive, 0).clipped_recently);
    }

    #[test]
    fn active_gain_lowers_reading_12_db() {
        let mut passive = DspPipeline::new(RATE);
        let mut active = DspPipeline::new(RATE);
        let x = bass_tone(55.0, 0.5, 0.2);
        let scaled: Vec<f32> = x.iter().map(|v| v * GAIN_ACTIVE).collect();
        feed_all(&mut passive, &x);
        feed_all(&mut active, &scaled);
        let d = passive.frame(BassMode::Passive, 0).peak_dbfs - active.frame(BassMode::Active, 0).peak_dbfs;
        assert!((d - 12.0).abs() < 0.1, "{d}");
    }

    #[test]
    fn mode_from_gain_and_serialization() {
        assert_eq!(BassMode::from_gain(GAIN_ACTIVE), BassMode::Active);
        assert_eq!(BassMode::from_gain(GAIN_PASSIVE), BassMode::Passive);
        assert_eq!(serde_json::to_string(&BassMode::Active).unwrap(), "\"active\"");
    }
}
