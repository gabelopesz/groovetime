#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod audio;
pub mod dsp;

pub fn run() {
    env_logger::init();

    if let Err(err) = tauri::Builder::default().run(tauri::generate_context!()) {
        log::error!("erro ao executar o Groove Time: {err}");
        std::process::exit(1);
    }
}
