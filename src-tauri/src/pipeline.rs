//! Pipeline de DSP (sem audio/Tauri): pico na taxa cheia, decimacao, YIN e
//! estabilizacao, agregados em `SignalFrame`.

use serde::Serialize;

use crate::audio::capture::{DEFAULT_GATE_DBFS, GAIN_ACTIVE, GAIN_PASSIVE};
use crate::dsp::note::DEFAULT_A4_HZ;
use crate::dsp::peak::FLOOR_DBFS;
use crate::dsp::rms::RMS_WINDOW_SECS;
use crate::dsp::dc::DC_WINDOW_SECS;
use crate::dsp::{
    ClipCounter, DcOffset, Decimator, MAX_PITCH_HZ, MIN_PITCH_HZ, NoiseFloor, PITCH_SAMPLE_RATE,
    PeakMeter, PitchSmoother, RmsMeter, YIN_WINDOW, Yin, freq_to_note,
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
    /// RMS (~300 ms) em dBFS.
    pub rms_dbfs: f32,
    /// Eventos de clip (cruzamentos de -3 dBFS) desde o ultimo reset.
    pub clip_count: u32,
    /// Ruido de fundo (dBFS); `None` ate haver ~2 s de silencio.
    pub noise_floor_dbfs: Option<f32>,
    /// DC offset em % de full scale.
    pub dc_offset: f32,
    /// Nivel abaixo do gate: a nota fica escondida.
    pub gated: bool,
    /// Referencia A4 (Hz) usada para nota/cents.
    pub reference_hz: f32,
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
    rms: RmsMeter,
    clip: ClipCounter,
    noise: NoiseFloor,
    dc: DcOffset,
    gate_dbfs: f32,
    rms_dbfs: f32,
    /// Duracao de audio processada desde o ultimo frame.
    elapsed_secs: f32,
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
            rms: RmsMeter::new(sample_rate, RMS_WINDOW_SECS),
            clip: ClipCounter::new(),
            noise: NoiseFloor::new(),
            dc: DcOffset::new(sample_rate, DC_WINDOW_SECS),
            gate_dbfs: DEFAULT_GATE_DBFS,
            rms_dbfs: FLOOR_DBFS,
            elapsed_secs: 0.0,
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
        self.gate_dbfs = gate_dbfs;
        self.yin.set_gate_dbfs(gate_dbfs);
    }

    pub fn reset_clips(&mut self) {
        self.clip.reset();
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
        let dt = block.len() as f32 / self.sample_rate as f32;
        if reading.over_threshold {
            self.clip_remaining_secs = CLIP_HOLD_SECS;
        } else {
            self.clip_remaining_secs = (self.clip_remaining_secs - dt).max(0.0);
        }
        self.clip.update(reading.peak_dbfs, dt);
        self.rms_dbfs = self.rms.process(block);
        self.dc.process(block);
        self.elapsed_secs += dt;

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
        // Gate: abaixo do limiar a nota some (RMS e diagnostico continuam).
        let gated = self.rms_dbfs < self.gate_dbfs;
        let pitch = smoothed.filter(|_| !gated).and_then(|freq_hz| {
            freq_to_note(freq_hz, self.a4_hz).map(|n| PitchInfo {
                freq_hz,
                note: n.name,
                octave: n.octave,
                cents: n.cents,
            })
        });

        let quiet = estimate.is_none() && gated;
        self.noise.update(self.rms_dbfs, quiet, self.elapsed_secs);
        self.elapsed_secs = 0.0;

        let frame = SignalFrame {
            peak_dbfs: self.interval_peak_dbfs,
            peak_hold_dbfs: self.hold_dbfs,
            over_threshold: self.interval_over,
            clipped_recently: self.clip_remaining_secs > 0.0,
            pitch,
            rms_dbfs: self.rms_dbfs,
            clip_count: self.clip.count(),
            noise_floor_dbfs: self.noise.value(),
            dc_offset: self.dc.percent(),
            gated,
            reference_hz: self.a4_hz,
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
    fn rms_clip_count_and_dc_in_frame() {
        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &bass_tone(55.0, 0.95, 0.6));
        let f = p.frame(BassMode::Passive, 0);
        assert_eq!(f.clip_count, 1);
        assert!(f.rms_dbfs < -3.0 && f.rms_dbfs > -12.0, "{}", f.rms_dbfs);
        assert!(f.dc_offset.abs() < 1.0, "{}", f.dc_offset);
        p.reset_clips();
        assert_eq!(p.frame(BassMode::Passive, 0).clip_count, 0);
    }

    #[test]
    fn gate_hides_the_note_and_is_adjustable() {
        let mut p = DspPipeline::new(RATE);
        // Tom a ~ -26 dBFS RMS: acima do gate padrao (-50).
        feed_all(&mut p, &bass_tone(55.0, 0.06, 0.6));
        let f = p.frame(BassMode::Passive, 0);
        assert!(!f.gated && f.pitch.is_some());
        p.set_gate_dbfs(-20.0);
        let f = p.frame(BassMode::Passive, 0);
        assert!(f.gated && f.pitch.is_none());
        p.set_gate_dbfs(-50.0);
        assert!(!p.frame(BassMode::Passive, 0).gated);
    }

    #[test]
    fn noise_floor_appears_after_silence_and_is_none_with_tone() {
        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &bass_tone(55.0, 0.5, 0.6));
        assert!(p.frame(BassMode::Passive, 0).noise_floor_dbfs.is_none());
        // 6 s de "silencio" com ruido muito baixo (o RMS de 300 ms leva ~3 s para cair abaixo do gate).
        let hiss: Vec<f32> = (0..384).map(|i| if i % 2 == 0 { 3e-4 } else { -3e-4 }).collect();
        let mut last = None;
        for _ in 0..((6.0 * RATE as f32 / 384.0) as usize) {
            p.feed(&hiss);
            if p.elapsed_secs >= 0.016 {
                last = p.frame(BassMode::Passive, 0).noise_floor_dbfs;
            }
        }
        let v = last.expect("noise floor");
        assert!((v + 70.5).abs() < 2.0, "{v}");
    }

    #[test]
    fn reference_pitch_changes_note_and_cents() {
        // 54 Hz: A1 exato com A4 = 432; ~ -32 cents com A4 = 440.
        let mut p = DspPipeline::new(RATE);
        p.set_a4_hz(432.0);
        feed_all(&mut p, &bass_tone(54.0, 0.5, 0.6));
        let f = p.frame(BassMode::Passive, 0);
        let a = f.pitch.expect("pitch");
        assert_eq!((a.note, a.octave), ("A", 1));
        assert!(a.cents.abs() < 2.0, "{}", a.cents);
        assert_eq!(f.reference_hz, 432.0);

        let mut p = DspPipeline::new(RATE);
        feed_all(&mut p, &bass_tone(54.0, 0.5, 0.6));
        let a = p.frame(BassMode::Passive, 0).pitch.expect("pitch");
        assert_eq!((a.note, a.octave), ("A", 1));
        assert!((a.cents + 31.8).abs() < 2.0, "{}", a.cents);
    }

    #[test]
    fn mode_from_gain_and_serialization() {
        assert_eq!(BassMode::from_gain(GAIN_ACTIVE), BassMode::Active);
        assert_eq!(BassMode::from_gain(GAIN_PASSIVE), BassMode::Passive);
        assert_eq!(serde_json::to_string(&BassMode::Active).unwrap(), "\"active\"");
    }
}
