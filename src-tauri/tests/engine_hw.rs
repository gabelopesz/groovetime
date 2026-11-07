//! Engine real no device default: `cargo test --test engine_hw -- --ignored --nocapture`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use groove_time_lib::audio::SharedControls;
use groove_time_lib::engine::EngineHandle;
use groove_time_lib::pipeline::{BassMode, SignalFrame};

#[test]
#[ignore = "exige um device de captura"]
fn emits_about_60_frames_per_second_and_shuts_down() {
    let controls = Arc::new(SharedControls::new(0));
    let count = Arc::new(AtomicUsize::new(0));
    let last: Arc<Mutex<Option<SignalFrame>>> = Arc::new(Mutex::new(None));
    let (c, l) = (Arc::clone(&count), Arc::clone(&last));
    let mut engine = EngineHandle::start(
        Arc::clone(&controls),
        None,
        move |f| {
            c.fetch_add(1, Ordering::Relaxed);
            *l.lock().unwrap() = Some(f.clone());
        },
        |msg| panic!("erro do engine: {msg}"),
    )
    .expect("engine");

    std::thread::sleep(Duration::from_millis(500));
    let start_count = count.load(Ordering::Relaxed);
    let t = Instant::now();
    std::thread::sleep(Duration::from_secs(3));
    let fps = (count.load(Ordering::Relaxed) - start_count) as f64 / t.elapsed().as_secs_f64();
    println!("{fps:.1} frames/s; info={:?}", engine.info);

    controls.set_active_mode(true);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(last.lock().unwrap().as_ref().unwrap().mode, BassMode::Active);

    let t = Instant::now();
    engine.stop();
    println!("stop em {:?}", t.elapsed());
    assert!((55.0..=65.0).contains(&fps), "fps fora da faixa: {fps}");
    assert_eq!(controls.dropped(), 0);
}

#[test]
#[ignore = "exige o host ALSA"]
fn missing_device_is_an_error_not_a_panic() {
    let controls = Arc::new(SharedControls::new(0));
    let result = EngineHandle::start(controls, Some("zzz-inexistente"), |_| {}, |_| {});
    assert!(result.is_err());
}
