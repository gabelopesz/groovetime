use thiserror::Error;

/// Erros do modulo de audio.
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("host de audio ALSA indisponivel: {0}")]
    HostUnavailable(#[source] cpal::Error),
    #[error("nenhum device de captura encontrado")]
    NoInputDevice,
    #[error("device de captura nao encontrado: {0}")]
    DeviceNotFound(String),
    #[error("nenhuma configuracao de captura suportada pelo device")]
    UnsupportedConfig,
    #[error("erro do backend de audio: {0}")]
    Backend(#[from] cpal::Error),
}
