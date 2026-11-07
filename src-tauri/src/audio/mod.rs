//! Captura de audio: selecao de device e configuracao.

pub mod capture;
pub mod device;
pub mod error;

pub use capture::{
    Capture, DEFAULT_GATE_DBFS, GAIN_ACTIVE, GATE_MAX_DBFS, GATE_MIN_DBFS, GAIN_PASSIVE, SharedControls, process_interleaved, start_capture,
};
pub use device::{
    CaptureConfig, DeviceInfo, StreamInfo, choose_config, find_device, list_input_devices,
    select_host,
};
pub use error::AudioError;
