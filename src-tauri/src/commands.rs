//! Commands Tauri expostos ao frontend (superficie minima).

use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};
use tauri::{AppHandle, Manager, State};

use crate::audio::{
    self, A4_MAX_HZ, A4_MIN_HZ, AudioError, DeviceInfo, GATE_MAX_DBFS, GATE_MIN_DBFS,
};
use crate::engine::{AppState, EngineState, EngineStatus};
use crate::pipeline::BassMode;
use crate::settings::{Settings, TunerView, is_valid_preset_id};
use crate::window::WindowState;

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
    state.persist(|s| s.bass_mode = mode);
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
    state.persist(|s| s.gate_dbfs = value);
    Ok(())
}

/// Referencia A4 em Hz; valida em `A4_MIN_HZ..=A4_MAX_HZ`.
#[tauri::command]
pub fn set_reference_pitch(hz: f32, state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    validate_reference(hz)?;
    state.controls.set_reference_hz(hz);
    state.persist(|s| s.reference_hz = hz);
    Ok(())
}

fn validate_reference(hz: f32) -> Result<(), CommandError> {
    if hz.is_finite() && (A4_MIN_HZ..=A4_MAX_HZ).contains(&hz) {
        Ok(())
    } else {
        Err(CommandError::InvalidArgument(format!(
            "A4 deve estar entre {A4_MIN_HZ} e {A4_MAX_HZ} Hz"
        )))
    }
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

/// Preferencias salvas (para o frontend hidratar preset, visualizacao etc.).
#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, CommandError> {
    Ok(state.settings())
}

/// Preferencias so de UI (preset e visualizacao); campo ausente (`null`) fica como esta.
/// O modo mini tem command proprio (`set_mini_mode`), pois mexe na janela.
#[derive(Debug, Deserialize)]
pub struct UiPrefs {
    pub tuning_preset: Option<String>,
    pub tuner_view: Option<TunerView>,
}

#[tauri::command]
pub fn set_ui_prefs(prefs: UiPrefs, state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    if let Some(preset) = prefs.tuning_preset.as_deref()
        && !is_valid_preset_id(preset)
    {
        return Err(CommandError::InvalidArgument("id de afinacao invalido".into()));
    }
    state.persist(|s| {
        if let Some(preset) = prefs.tuning_preset {
            s.tuning_preset = preset;
        }
        if let Some(view) = prefs.tuner_view {
            s.tuner_view = view;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn get_window_state(state: State<'_, Arc<AppState>>) -> Result<WindowState, CommandError> {
    Ok(state.window_state())
}

/// Entra/sai do modo mini (janela compacta, sempre no topo) e persiste a escolha.
/// Feito no Rust: o frontend nao precisa de permissoes `core:window:*`.
#[tauri::command]
pub async fn set_mini_mode(
    enabled: bool,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<WindowState, CommandError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| CommandError::Internal("janela principal nao encontrada".into()))?;
    state
        .apply_mini_mode(&window, enabled)
        .map_err(|e| CommandError::Internal(e.to_string()))?;
    state.persist(|s| s.mini_mode = enabled);
    Ok(state.window_state())
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
    tauri::async_runtime::spawn_blocking(move || {
        let status = state.start(&app, device.as_deref(), true);
        // So lembra a preferencia se o device realmente abriu.
        if status.state == EngineState::Running {
            state.persist(|s| s.device_name = device.clone());
        }
        status
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_validation() {
        for ok in [432.0, 440.0, 446.0] {
            assert!(validate_reference(ok).is_ok(), "{ok}");
        }
        for bad in [500.0, 431.9, 446.1, 0.0, f32::NAN, f32::INFINITY] {
            assert!(validate_reference(bad).is_err(), "{bad}");
        }
    }

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
