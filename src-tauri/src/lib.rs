#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod audio;
mod commands;
pub mod dsp;
pub mod engine;
pub mod pipeline;
pub mod settings;

use std::sync::Arc;

use tauri::{Manager, RunEvent};

use engine::AppState;

pub fn run() {
    env_logger::init();

    let state = Arc::new(AppState::new());
    let setup_state = Arc::clone(&state);

    let built = tauri::Builder::default()
        .manage(Arc::clone(&state))
        .setup(move |app| {
            // Preferencias: carregar e aplicar ANTES de iniciar o engine.
            match app.path().app_config_dir() {
                Ok(dir) => setup_state.init_settings(settings::SettingsStore::open(dir.join("settings.json"))),
                Err(e) => log::error!("sem diretorio de config ({e}); preferencias nao serao salvas"),
            }
            let saved = setup_state.settings();
            setup_state.controls.set_active_mode(saved.bass_mode == pipeline::BassMode::Active);
            setup_state.controls.set_reference_hz(saved.reference_hz);
            setup_state.controls.set_gate_dbfs(saved.gate_dbfs);

            // Enumerar devices ALSA pode demorar: nao bloquear a abertura da janela.
            // Preferencia de device inexistente cai para o automatico (sem sobrescrever o salvo).
            let handle = app.handle().clone();
            let state = Arc::clone(&setup_state);
            std::thread::spawn(move || {
                state.start(&handle, saved.device_name.as_deref(), false);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_bass_mode,
            commands::reset_clip_count,
            commands::set_gate_dbfs,
            commands::set_reference_pitch,
            commands::get_settings,
            commands::set_ui_prefs,
            commands::get_engine_status,
            commands::list_input_devices,
            commands::restart_engine,
        ])
        .build(tauri::generate_context!());

    match built {
        Ok(app) => app.run(move |_handle, event| {
            if let RunEvent::Exit = event {
                state.stop();
                state.flush_settings();
            }
        }),
        Err(err) => {
            log::error!("erro ao construir o Groove Time: {err}");
            std::process::exit(1);
        }
    }
}
