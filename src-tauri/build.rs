fn main() {
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_bass_mode",
            "reset_clip_count",
            "set_gate_dbfs",
            "set_reference_pitch",
            "get_engine_status",
            "get_settings",
            "get_window_state",
            "set_mini_mode",
            "set_ui_prefs",
            "list_input_devices",
            "restart_engine",
        ]),
    );
    if let Err(err) = tauri_build::try_build(attributes) {
        eprintln!("erro no tauri-build: {err:#}");
        std::process::exit(1);
    }
}
