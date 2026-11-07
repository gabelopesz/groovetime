//! Commands Tauri expostos ao frontend (superficie minima).

use std::sync::Arc;

use serde::{Serialize, Serializer};
use tauri::{AppHandle, State};

use crate::audio::{self, AudioError, DeviceInfo};
use crate::engine::{AppState, EngineStatus};
use crate::pipeline::BassMode;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Audio(#[from] AudioError),
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
