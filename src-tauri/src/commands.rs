//! Commands Tauri expostos ao frontend (superficie minima).

use std::sync::Arc;

use serde::{Serialize, Serializer};
use tauri::{AppHandle, State};

use crate::audio::{self, AudioError, DeviceInfo, GATE_MAX_DBFS, GATE_MIN_DBFS};
use crate::engine::{AppState, EngineStatus};
use crate::pipeline::BassMode;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("valor invalido: {0}")]
    InvalidArgument(String),
    #[error("falha interna: {0}")]
    Internal(String),
}

impl Serialize for CommandError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[tauri::command]
pub fn set_bass_mode(mode: BassMode, state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    state.controls.set_active_mode(mode == BassMode::Active);
    Ok(())
}

#[tauri::command]
pub fn reset_clip_count(state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    state.controls.request_clip_reset();
    Ok(())
}

/// Gate de nivel em dBFS; valido em `GATE_MIN_DBFS..=GATE_MAX_DBFS`.
#[tauri::command]
pub fn set_gate_dbfs(value: f32, state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    validate_gate(value)?;
    state.controls.set_gate_dbfs(value);
    Ok(())
}

fn validate_gate(value: f32) -> Result<(), CommandError> {
    if value.is_finite() && (GATE_MIN_DBFS..=GATE_MAX_DBFS).contains(&value) {
        Ok(())
    } else {
        Err(CommandError::InvalidArgument(format!(
            "gate deve estar entre {GATE_MIN_DBFS} e {GATE_MAX_DBFS} dBFS"
        )))
    }
}

#[tauri::command]
pub fn get_engine_status(state: State<'_, Arc<AppState>>) -> Result<EngineStatus, CommandError> {
    Ok(state.status())
}

#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<DeviceInfo>, CommandError> {
    tauri::async_runtime::spawn_blocking(audio::list_input_devices)
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))?
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn restart_engine(
    device: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<EngineStatus, CommandError> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || state.start(&app, device.as_deref()))
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_validation() {
        assert!(validate_gate(-50.0).is_ok());
        assert!(validate_gate(-80.0).is_ok());
        assert!(validate_gate(-20.0).is_ok());
        assert!(validate_gate(-200.0).is_err());
        assert!(validate_gate(0.0).is_err());
        assert!(validate_gate(f32::NAN).is_err());
        assert!(validate_gate(f32::INFINITY).is_err());
    }
}
