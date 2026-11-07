//! Lista os devices de captura e a configuracao escolhida: `cargo run --example list_devices`.

use groove_time_lib::audio::{self, StreamInfo};

fn main() {
    env_logger::init();
    match audio::list_input_devices() {
        Ok(devices) => {
            for d in &devices {
                println!(
                    "{} [{}] canais={} rates={:?}",
                    d.name, d.id, d.max_channels, d.sample_rate_ranges
                );
            }
        }
        Err(e) => eprintln!("erro ao listar devices: {e}"),
    }
    let preference = std::env::args().nth(1);
    match audio::find_device(preference.as_deref()) {
        Ok(device) => match audio::choose_config(&device, 0) {
            Ok(cfg) => {
                let name = audio::device::device_name(&device);
                println!("escolhido: {:?}", StreamInfo::new(name, &cfg));
            }
            Err(e) => eprintln!("erro ao escolher config: {e}"),
        },
        Err(e) => eprintln!("erro ao achar device: {e}"),
    }
}
