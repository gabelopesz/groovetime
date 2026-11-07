//! Captura de audio: selecao de device e configuracao.

pub mod device;
pub mod error;

pub use device::{
    CaptureConfig, DeviceInfo, StreamInfo, choose_config, find_device, list_input_devices,
    select_host,
};
pub use error::AudioError;
