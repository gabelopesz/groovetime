//! Captura em tempo real: callback lock-free que extrai o canal alvo, aplica o
//! ganho (Ativo/Passivo) e envia amostras f32 para um ring buffer SPSC.
//!
//! Regras da callback de dados: sem locks, sem alocacao, sem log, sem
//! `unwrap`/`expect`. Toda comunicacao com o resto do app e por atomicos.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, FromSample, I24, Sample, SampleFormat, SizedSample, Stream};
use rtrb::{Consumer, Producer, RingBuffer};

use super::device::CaptureConfig;
use super::error::AudioError;

/// Ganho do modo Passivo (sem atenuacao).
pub const GAIN_PASSIVE: f32 = 1.0;
/// Ganho do modo Ativo: -12 dB (`10^(-12/20)`).
pub const GAIN_ACTIVE: f32 = 0.251_188_64;
/// Gate padrao e faixa valida (dBFS).
pub const DEFAULT_GATE_DBFS: f32 = -50.0;
pub const GATE_MIN_DBFS: f32 = -80.0;
pub const GATE_MAX_DBFS: f32 = -20.0;
/// Duracao do ring buffer, em segundos de audio.
const RING_SECONDS: f32 = 0.5;

/// Estado compartilhado entre a callback RT e o resto do app (apenas atomicos).
#[derive(Debug)]
pub struct SharedControls {
    gain_bits: AtomicU32,
    channel_index: AtomicUsize,
    gate_bits: AtomicU32,
    reset_clips: AtomicBool,
    /// Amostras descartadas por ring cheio.
    pub dropped_samples: AtomicU64,
    /// Erro de stream (device removido, formato inesperado, ...).
    pub stream_error: AtomicBool,
}

impl SharedControls {
    pub fn new(target_channel: usize) -> Self {
        Self {
            gain_bits: AtomicU32::new(GAIN_PASSIVE.to_bits()),
            channel_index: AtomicUsize::new(target_channel),
            gate_bits: AtomicU32::new(DEFAULT_GATE_DBFS.to_bits()),
            reset_clips: AtomicBool::new(false),
            dropped_samples: AtomicU64::new(0),
            stream_error: AtomicBool::new(false),
        }
    }

    pub fn gain(&self) -> f32 {
        f32::from_bits(self.gain_bits.load(Ordering::Relaxed))
    }

    pub fn set_gain(&self, gain: f32) {
        self.gain_bits.store(gain.to_bits(), Ordering::Relaxed);
    }

    /// `true` = Ativo (-12 dB), `false` = Passivo (0 dB).
    pub fn set_active_mode(&self, active: bool) {
        self.set_gain(if active { GAIN_ACTIVE } else { GAIN_PASSIVE });
    }

    pub fn channel(&self) -> usize {
        self.channel_index.load(Ordering::Relaxed)
    }

    pub fn set_channel(&self, channel: usize) {
        self.channel_index.store(channel, Ordering::Relaxed);
    }

    /// Gate de nivel (dBFS) lido pela thread DSP.
    pub fn gate_dbfs(&self) -> f32 {
        f32::from_bits(self.gate_bits.load(Ordering::Relaxed))
    }

    pub fn set_gate_dbfs(&self, gate_dbfs: f32) {
        self.gate_bits.store(gate_dbfs.to_bits(), Ordering::Relaxed);
    }

    /// Pede o reset do contador de clips (consumido pela thread DSP).
    pub fn request_clip_reset(&self) {
        self.reset_clips.store(true, Ordering::Relaxed);
    }

    /// Consome o pedido de reset (`true` uma unica vez por pedido).
    pub fn take_clip_reset(&self) -> bool {
        self.reset_clips.swap(false, Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped_samples.load(Ordering::Relaxed)
    }

    pub fn has_stream_error(&self) -> bool {
        self.stream_error.load(Ordering::Relaxed)
    }
}

/// Captura em andamento. Soltar a struct encerra o stream.
pub struct Capture {
    /// Mantem o stream vivo (`cpal::Stream` e `Send + Sync` no ALSA).
    pub stream: Stream,
    pub consumer: Consumer<f32>,
    pub controls: Arc<SharedControls>,
    /// Configuracao efetivamente usada (pode diferir da pedida se houve fallback).
    pub config: CaptureConfig,
}

/// Logica pura da callback: extrai `channel` de cada frame intercalado, converte
/// para f32, aplica `gain` e faz `push`. Em ring cheio conta o drop e segue.
/// Sem alocacao, locks ou panics.
pub fn process_interleaved<T>(
    data: &[T],
    channels: usize,
    channel: usize,
    gain: f32,
    producer: &mut Producer<f32>,
    dropped: &AtomicU64,
) where
    T: Sample,
    f32: FromSample<T>,
{
    if channels == 0 {
        return;
    }
    let channel = channel.min(channels - 1);
    let mut drops = 0u64;
    for frame in data.chunks_exact(channels) {
        if let Some(&sample) = frame.get(channel) {
            let x = f32::from_sample_(sample) * gain;
            if producer.push(x).is_err() {
                drops += 1;
            }
        }
    }
    if drops > 0 {
        dropped.fetch_add(drops, Ordering::Relaxed);
    }
}

/// Constroi o stream para o tipo de amostra `T`.
fn build_stream<T>(
    device: &Device,
    config: &CaptureConfig,
    controls: &Arc<SharedControls>,
    mut producer: Producer<f32>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let data_controls = Arc::clone(controls);
    let error_controls = Arc::clone(controls);
    device.build_input_stream_raw(
        config.stream_config,
        T::FORMAT,
        // --- callback RT: sem locks, alocacao, log ou unwrap/expect ---
        move |data, _| match data.as_slice::<T>() {
            Some(samples) => process_interleaved(
                samples,
                channels,
                data_controls.channel(),
                data_controls.gain(),
                &mut producer,
                &data_controls.dropped_samples,
            ),
            None => data_controls.stream_error.store(true, Ordering::Relaxed),
        },
        move |_err| error_controls.stream_error.store(true, Ordering::Relaxed),
        // --- fim da callback RT ---
        None,
    )
}

/// Cria ring + stream para `config` e inicia a captura.
fn try_start(
    device: &Device,
    config: &CaptureConfig,
    controls: &Arc<SharedControls>,
) -> Result<(Stream, Consumer<f32>), AudioError> {
    let capacity = ((config.sample_rate as f32 * RING_SECONDS) as usize).max(1024);
    let (producer, consumer) = RingBuffer::<f32>::new(capacity);

    let stream = match config.sample_format {
        SampleFormat::I16 => build_stream::<i16>(device, config, controls, producer)?,
        SampleFormat::I24 => build_stream::<I24>(device, config, controls, producer)?,
        SampleFormat::I32 => build_stream::<i32>(device, config, controls, producer)?,
        SampleFormat::F32 => build_stream::<f32>(device, config, controls, producer)?,
        SampleFormat::F64 => build_stream::<f64>(device, config, controls, producer)?,
        _ => return Err(AudioError::UnsupportedConfig),
    };
    stream.play()?;
    Ok((stream, consumer))
}

/// Inicia a captura. Se a config escolhida falhar ao abrir (comum em PCMs-plugin
/// do ALSA que anunciam faixas irreais), cai para a config default do device.
pub fn start_capture(
    device: &Device,
    config: CaptureConfig,
    controls: Arc<SharedControls>,
) -> Result<Capture, AudioError> {
    controls.stream_error.store(false, Ordering::Relaxed);
    match try_start(device, &config, &controls) {
        Ok((stream, consumer)) => {
            log::info!(
                "captura iniciada: {} Hz, {} canais, {:?}, canal alvo {}",
                config.sample_rate,
                config.channels,
                config.sample_format,
                config.target_channel
            );
            Ok(Capture { stream, consumer, controls, config })
        }
        Err(first) => {
            log::warn!("falha ao abrir a config escolhida ({first}); tentando a default do device");
            let default = device.default_input_config()?;
            let fallback = CaptureConfig {
                stream_config: default.config(),
                sample_format: default.sample_format(),
                channels: default.channels(),
                sample_rate: default.sample_rate(),
                target_channel: config.target_channel,
            };
            let (stream, consumer) = try_start(device, &fallback, &controls)?;
            log::info!(
                "captura iniciada (fallback): {} Hz, {} canais, {:?}",
                fallback.sample_rate,
                fallback.channels,
                fallback.sample_format
            );
            Ok(Capture { stream, consumer, controls, config: fallback })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(n: usize) -> (Producer<f32>, Consumer<f32>) {
        RingBuffer::<f32>::new(n)
    }

    fn drain(c: &mut Consumer<f32>) -> Vec<f32> {
        let mut v = Vec::new();
        while let Ok(x) = c.pop() {
            v.push(x);
        }
        v
    }

    #[test]
    fn extracts_channel_and_applies_gain_f32() {
        let (mut p, mut c) = ring(16);
        let dropped = AtomicU64::new(0);
        // Estereo: L = 0.5, 0.25 ; R = -1.0, -0.5
        let data = [0.5f32, -1.0, 0.25, -0.5];
        process_interleaved(&data, 2, 0, 1.0, &mut p, &dropped);
        assert_eq!(drain(&mut c), vec![0.5, 0.25]);
        process_interleaved(&data, 2, 1, 0.5, &mut p, &dropped);
        assert_eq!(drain(&mut c), vec![-0.5, -0.25]);
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn converts_integer_formats() {
        let (mut p, mut c) = ring(16);
        let dropped = AtomicU64::new(0);
        process_interleaved(&[i16::MAX, 0, i16::MIN, 0], 2, 0, 1.0, &mut p, &dropped);
        let out = drain(&mut c);
        assert!((out[0] - 1.0).abs() < 1e-3 && (out[1] + 1.0).abs() < 1e-3);
        process_interleaved(&[i32::MAX / 2, 0], 2, 0, GAIN_ACTIVE, &mut p, &dropped);
        let out = drain(&mut c);
        assert!((out[0] - 0.5 * GAIN_ACTIVE).abs() < 1e-3);
    }

    #[test]
    fn counts_drops_when_full_and_ignores_trailing_partial_frame() {
        let (mut p, mut c) = ring(2);
        let dropped = AtomicU64::new(0);
        // 5 frames mono, ring de 2 -> 3 drops.
        process_interleaved(&[0.1f32; 5], 1, 0, 1.0, &mut p, &dropped);
        assert_eq!(dropped.load(Ordering::Relaxed), 3);
        assert_eq!(drain(&mut c).len(), 2);
        // Frame incompleto no fim e descartado sem panic.
        process_interleaved(&[0.1f32; 5], 2, 0, 1.0, &mut p, &dropped);
        assert_eq!(drain(&mut c).len(), 2);
    }

    #[test]
    fn out_of_range_channel_is_clamped_and_zero_channels_is_safe() {
        let (mut p, mut c) = ring(8);
        let dropped = AtomicU64::new(0);
        process_interleaved(&[0.1f32, 0.9], 2, 7, 1.0, &mut p, &dropped);
        assert_eq!(drain(&mut c), vec![0.9]);
        process_interleaved(&[0.1f32], 0, 0, 1.0, &mut p, &dropped);
        assert!(drain(&mut c).is_empty());
    }

    #[test]
    fn controls_gate_and_clip_reset() {
        let c = SharedControls::new(0);
        assert_eq!(c.gate_dbfs(), DEFAULT_GATE_DBFS);
        c.set_gate_dbfs(-42.5);
        assert_eq!(c.gate_dbfs(), -42.5);
        assert!(!c.take_clip_reset());
        c.request_clip_reset();
        assert!(c.take_clip_reset());
        assert!(!c.take_clip_reset());
    }

    #[test]
    fn controls_gain_and_mode() {
        let c = SharedControls::new(0);
        assert_eq!(c.gain(), GAIN_PASSIVE);
        c.set_active_mode(true);
        assert!((20.0 * c.gain().log10() + 12.0).abs() < 0.01);
        c.set_active_mode(false);
        assert_eq!(c.gain(), 1.0);
    }
}
