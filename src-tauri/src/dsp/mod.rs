//! DSP puro (sem audio/Tauri): pico em dBFS, decimacao, YIN e conversao de notas.

pub mod decimate;
pub mod note;
pub mod peak;
pub mod smooth;
pub mod yin;

pub use decimate::Decimator;
pub use note::{NoteInfo, freq_to_note, note_to_freq};
pub use peak::{PEAK_ALERT_DBFS, PeakMeter, PeakReading};
pub use smooth::PitchSmoother;
pub use yin::{PitchEstimate, Yin};

/// Taxa alvo (Hz) do sinal decimado que alimenta o YIN.
pub const PITCH_SAMPLE_RATE: u32 = 8_000;
/// Janela do YIN em amostras na taxa decimada (~128 ms a 8 kHz).
pub const YIN_WINDOW: usize = 1024;
/// Menor frequencia detectavel (margem abaixo de B0 = 30.87 Hz).
pub const MIN_PITCH_HZ: f32 = 28.0;
/// Maior frequencia detectavel.
pub const MAX_PITCH_HZ: f32 = 500.0;
