#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Teste de integracao com hardware/PipeWire: `cargo test --test capture_hw -- --ignored`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use groove_time_lib::audio::{self, SharedControls};

#[test]
#[ignore = "exige um device de captura"]
fn captures_samples_without_drops() {
    let device = audio::find_device(None).expect("device");
    let config = audio::choose_config(&device, 0).expect("config");
    let controls = Arc::new(SharedControls::new(0));
    let mut capture = audio::start_capture(&device, config, Arc::clone(&controls)).expect("capture");

    let (mut received, start) = (0usize, Instant::now());
    while start.elapsed() < Duration::from_secs(2) {
        while capture.consumer.pop().is_ok() {
            received += 1;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("recebidas {received} amostras a {} Hz", capture.config.sample_rate);
    assert!(received > 0, "nenhuma amostra recebida");
    assert_eq!(controls.dropped(), 0);
    assert!(!controls.has_stream_error());
}
