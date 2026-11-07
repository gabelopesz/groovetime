//! Captura de audio: selecao de device e configuracao.

pub mod capture;
pub mod device;
pub mod error;

pub use capture::{
    A4_MAX_HZ, A4_MIN_HZ, Capture, DEFAULT_A4_HZ, DEFAULT_GATE_DBFS, GAIN_ACTIVE, GATE_MAX_DBFS, GATE_MIN_DBFS, GAIN_PASSIVE, SharedControls, process_interleaved, start_capture,
};
pub use device::{
    CaptureConfig, DeviceInfo, StreamInfo, choose_config, find_device, find_device_or_default, list_input_devices,
    select_host,
};
pub use error::AudioError;
