pub mod audio;

pub fn run() {
    env_logger::init();

    if let Err(err) = tauri::Builder::default().run(tauri::generate_context!()) {
        log::error!("erro ao executar o Groove Time: {err}");
        std::process::exit(1);
    }
}
