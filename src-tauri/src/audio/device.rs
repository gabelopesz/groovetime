//! Enumeracao de devices de captura e escolha da configuracao (host ALSA).

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host, HostId, SampleFormat, SampleRate, StreamConfig};
use serde::Serialize;

use super::error::AudioError;

/// Taxa de amostragem alvo.
pub const TARGET_SAMPLE_RATE: SampleRate = 192_000;

/// Trechos de nome usados para localizar a Teyun Q-24 quando nao ha preferencia.
const DEFAULT_NAME_HINTS: [&str; 3] = ["q-24", "q24", "teyun"];

/// Configuracao de captura escolhida (passada ao passo 4).
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub stream_config: StreamConfig,
    pub sample_format: SampleFormat,
    pub channels: u16,
    pub sample_rate: SampleRate,
    /// Indice do canal analisado (0 = Input 1 HI-Z).
    pub target_channel: u16,
}

impl CaptureConfig {
    /// Se a rate efetiva e menor que o alvo de 192 kHz.
    pub fn is_below_target_rate(&self) -> bool {
        self.sample_rate < TARGET_SAMPLE_RATE
    }
}

/// Informacao serializavel sobre a captura em andamento (para a UI).
#[derive(Debug, Clone, Serialize)]
pub struct StreamInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
    pub target_channel: u16,
    pub below_target_rate: bool,
}

impl StreamInfo {
    pub fn new(device_name: impl Into<String>, config: &CaptureConfig) -> Self {
        Self {
            device_name: device_name.into(),
            sample_rate: config.sample_rate,
            channels: config.channels,
            sample_format: format!("{:?}", config.sample_format),
            target_channel: config.target_channel,
            below_target_rate: config.is_below_target_rate(),
        }
    }
}

/// Descricao serializavel de um device de captura.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub max_channels: u16,
    /// Faixas `(min, max)` de sample rate suportadas.
    pub sample_rate_ranges: Vec<(u32, u32)>,
}

/// Candidata de configuracao, independente do cpal (permite testar sem hardware).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigCandidate {
    pub channels: u16,
    pub min_rate: SampleRate,
    pub max_rate: SampleRate,
    pub format: SampleFormat,
}

/// Prioridade do formato (menor = melhor). `None` = formato nao utilizavel.
fn format_rank(format: SampleFormat) -> Option<u8> {
    match format {
        SampleFormat::I32 | SampleFormat::I24 => Some(0),
        SampleFormat::F32 => Some(1),
        SampleFormat::I16 => Some(2),
        _ => None,
    }
}

/// Escolhe a melhor candidata e a rate a usar.
///
/// Criterios: (1) rate 192 kHz, senao a maior rate disponivel; (2) formato
/// I32/I24 > F32 > I16; (3) canais suficientes para `target_channel`.
/// Retorna `(indice, rate)`.
pub fn rank_configs(
    candidates: &[ConfigCandidate],
    target_channel: u16,
) -> Option<(usize, SampleRate)> {
    let usable = |c: &ConfigCandidate| c.channels > target_channel && format_rank(c.format).is_some();

    let best_rate = if candidates
        .iter()
        .any(|c| usable(c) && c.min_rate <= TARGET_SAMPLE_RATE && TARGET_SAMPLE_RATE <= c.max_rate)
    {
        TARGET_SAMPLE_RATE
    } else {
        candidates.iter().filter(|c| usable(c)).map(|c| c.max_rate).max()?
    };

    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| usable(c) && c.min_rate <= best_rate && best_rate <= c.max_rate)
        .min_by_key(|(_, c)| (format_rank(c.format).unwrap_or(u8::MAX), c.channels))
        .map(|(i, _)| (i, best_rate))
}

/// Seleciona o host ALSA.
pub fn select_host() -> Result<Host, AudioError> {
    cpal::host_from_id(HostId::Alsa).map_err(AudioError::HostUnavailable)
}

/// Nome legivel do device (ou o id, se a descricao falhar).
pub fn device_name(device: &Device) -> String {
    device
        .description()
        .map(|d| d.name().to_owned())
        .or_else(|_| device.id().map(|id| id.to_string()))
        .unwrap_or_else(|_| "<desconhecido>".to_owned())
}

/// Lista os devices de captura com canais e faixas de sample rate.
pub fn list_input_devices() -> Result<Vec<DeviceInfo>, AudioError> {
    let host = select_host()?;
    let mut out = Vec::new();
    for device in host.input_devices()? {
        let id = device.id().map(|i| i.to_string()).unwrap_or_default();
        let name = device_name(&device);
        let (mut max_channels, mut ranges) = (0u16, Vec::new());
        if let Ok(configs) = device.supported_input_configs() {
            for c in configs {
                max_channels = max_channels.max(c.channels());
                let range = (c.min_sample_rate(), c.max_sample_rate());
                if !ranges.contains(&range) {
                    ranges.push(range);
                }
            }
        }
        out.push(DeviceInfo { id, name, max_channels, sample_rate_ranges: ranges });
    }
    Ok(out)
}

/// Verifica (case-insensitive) se algum dos `hints` aparece em algum dos `haystacks`.
pub fn matches_any(haystacks: &[String], hints: &[&str]) -> bool {
    haystacks.iter().any(|h| {
        let h = h.to_lowercase();
        hints.iter().any(|hint| h.contains(&hint.to_lowercase()))
    })
}

fn device_haystacks(device: &Device) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(id) = device.id() {
        v.push(id.to_string());
    }
    if let Ok(desc) = device.description() {
        v.push(desc.name().to_owned());
        v.extend(desc.extended().map(str::to_owned));
    }
    v
}

/// Encontra o device de captura.
///
/// Com `preference`, exige correspondencia por substring (case-insensitive) no
/// nome/id (ex.: `"hw:"` para o modo exclusivo) e falha com `DeviceNotFound`.
/// Sem preferencia, tenta Q-24/Teyun e cai para o device default.
pub fn find_device(preference: Option<&str>) -> Result<Device, AudioError> {
    let host = select_host()?;
    let hints: Vec<&str> = match preference {
        Some(p) => vec![p],
        None => DEFAULT_NAME_HINTS.to_vec(),
    };
    for device in host.input_devices()? {
        if matches_any(&device_haystacks(&device), &hints) {
            return Ok(device);
        }
    }
    if let Some(p) = preference {
        return Err(AudioError::DeviceNotFound(p.to_owned()));
    }
    log::warn!("Q-24 nao encontrada; usando o device de captura default");
    host.default_input_device().ok_or(AudioError::NoInputDevice)
}

/// Como `find_device`, mas se a preferencia nao for encontrada (ex.: placa desconectada)
/// cai para o device automatico, sem tratar como erro.
pub fn find_device_or_default(preference: Option<&str>) -> Result<Device, AudioError> {
    match find_device(preference) {
        Err(AudioError::DeviceNotFound(p)) => {
            log::warn!("device preferido \"{p}\" nao encontrado; usando o automatico");
            find_device(None)
        }
        other => other,
    }
}

/// Escolhe a configuracao de captura do device (alvo 192 kHz, com fallback).
pub fn choose_config(device: &Device, target_channel: u16) -> Result<CaptureConfig, AudioError> {
    let ranges: Vec<_> = device.supported_input_configs()?.collect();
    let candidates: Vec<ConfigCandidate> = ranges
        .iter()
        .map(|c| ConfigCandidate {
            channels: c.channels(),
            min_rate: c.min_sample_rate(),
            max_rate: c.max_sample_rate(),
            format: c.sample_format(),
        })
        .collect();

    let (idx, rate) = rank_configs(&candidates, target_channel).ok_or(AudioError::UnsupportedConfig)?;
    let chosen = ranges
        .get(idx)
        .and_then(|r| r.try_with_sample_rate(rate))
        .ok_or(AudioError::UnsupportedConfig)?;

    if rate < TARGET_SAMPLE_RATE {
        log::warn!("{TARGET_SAMPLE_RATE} Hz indisponivel; usando {rate} Hz");
    }
    Ok(CaptureConfig {
        stream_config: chosen.config(),
        sample_format: chosen.sample_format(),
        channels: chosen.channels(),
        sample_rate: chosen.sample_rate(),
        target_channel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(channels: u16, min: u32, max: u32, format: SampleFormat) -> ConfigCandidate {
        ConfigCandidate { channels, min_rate: min, max_rate: max, format }
    }

    #[test]
    fn prefers_192k_over_other_rates() {
        let c = [
            cand(2, 8_000, 96_000, SampleFormat::I32),
            cand(2, 44_100, 192_000, SampleFormat::I32),
        ];
        assert_eq!(rank_configs(&c, 0), Some((1, 192_000)));
    }

    #[test]
    fn rate_beats_format() {
        let c = [
            cand(2, 48_000, 48_000, SampleFormat::I32),
            cand(2, 192_000, 192_000, SampleFormat::I16),
        ];
        assert_eq!(rank_configs(&c, 0), Some((1, 192_000)));
    }

    #[test]
    fn falls_back_to_highest_rate() {
        let c = [
            cand(2, 8_000, 48_000, SampleFormat::I32),
            cand(2, 8_000, 96_000, SampleFormat::F32),
        ];
        assert_eq!(rank_configs(&c, 0), Some((1, 96_000)));
    }

    #[test]
    fn prefers_i32_and_f32_over_i16() {
        let c = [
            cand(2, 192_000, 192_000, SampleFormat::I16),
            cand(2, 192_000, 192_000, SampleFormat::F32),
        ];
        assert_eq!(rank_configs(&c, 0), Some((1, 192_000)));
        let c = [
            cand(2, 192_000, 192_000, SampleFormat::F32),
            cand(2, 192_000, 192_000, SampleFormat::I32),
        ];
        assert_eq!(rank_configs(&c, 0), Some((1, 192_000)));
    }

    #[test]
    fn requires_enough_channels_and_known_format() {
        let c = [cand(1, 192_000, 192_000, SampleFormat::I32)];
        assert_eq!(rank_configs(&c, 1), None);
        let c = [cand(2, 192_000, 192_000, SampleFormat::U8)];
        assert_eq!(rank_configs(&c, 0), None);
        assert_eq!(rank_configs(&[], 0), None);
    }

    #[test]
    fn name_matching_is_case_insensitive() {
        let h = vec!["Teyun Q-24 USB".to_owned()];
        assert!(matches_any(&h, &DEFAULT_NAME_HINTS));
        assert!(matches_any(&h, &["TEYUN"]));
        assert!(!matches_any(&["HDA Intel PCH".to_owned()], &DEFAULT_NAME_HINTS));
    }
}
