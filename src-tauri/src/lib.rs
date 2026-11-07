#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod audio;
mod commands;
pub mod dsp;
pub mod engine;
pub mod pipeline;

use std::sync::Arc;

use tauri::RunEvent;

use engine::AppState;

pub fn run() {
    env_logger::init();

    let state = Arc::new(AppState::new());
    let setup_state = Arc::clone(&state);

    let built = tauri::Builder::default()
        .manage(Arc::clone(&state))
        .setup(move |app| {
            // Enumerar devices ALSA pode demorar: nao bloquear a abertura da janela.
            let handle = app.handle().clone();
            let state = Arc::clone(&setup_state);
            std::thread::spawn(move || {
                state.start(&handle, None);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_bass_mode,
            commands::get_engine_status,
            commands::list_input_devices,
            commands::restart_engine,
        ])
        .build(tauri::generate_context!());

    match built {
        Ok(app) => app.run(move |_handle, event| {
            if let RunEvent::Exit = event {
                state.stop();
            }
        }),
        Err(err) => {
            log::error!("erro ao construir o Groove Time: {err}");
            std::process::exit(1);
        }
    }
}
