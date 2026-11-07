//! Engine: thread de DSP que drena o ring buffer, alimenta o pipeline e emite
//! no maximo um `SignalFrame` a cada ~16 ms.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::Stream;
use cpal::traits::StreamTrait;
use rtrb::Consumer;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::audio::device::device_name;
use crate::audio::{self, AudioError, SharedControls, StreamInfo};
use crate::pipeline::{BassMode, DspPipeline, SignalFrame};
use crate::settings::{Settings, SettingsStore};

pub const SIGNAL_FRAME_EVENT: &str = "signal-frame";
pub const ENGINE_STATUS_EVENT: &str = "engine-status";

/// Intervalo entre frames (~60 fps).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
/// Espera quando nao ha trabalho (limitada pelo proximo frame).
const IDLE_SLEEP: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Starting,
    Running,
    Error,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub state: EngineState,
    pub message: Option<String>,
    pub stream: Option<StreamInfo>,
    pub mode: BassMode,
    pub gate_dbfs: f32,
    pub reference_hz: f32,
}

/// Handle de um engine em execucao. `stop` (ou drop) encerra a thread e o stream.
pub struct EngineHandle {
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Mantem o stream de captura vivo.
    stream: Option<Stream>,
    pub info: StreamInfo,
}

impl EngineHandle {
    /// Inicia captura + thread DSP. `on_frame` roda na thread DSP; `on_error`
    /// roda (uma vez) se o stream falhar. Com `strict`, uma preferencia de device
    /// inexistente e erro; sem `strict`, cai para o device automatico.
    pub fn start(
        controls: Arc<SharedControls>,
        device_pref: Option<&str>,
        strict: bool,
        on_frame: impl FnMut(&SignalFrame) + Send + 'static,
        on_error: impl FnOnce(String) + Send + 'static,
    ) -> Result<Self, AudioError> {
        let device = if strict {
            audio::find_device(device_pref)?
        } else {
            audio::find_device_or_default(device_pref)?
        };
        let name = device_name(&device);
        let config = audio::choose_config(&device, 0)?;
        let capture = audio::start_capture(&device, config, Arc::clone(&controls))?;

        let audio::Capture { stream, consumer, config, .. } = capture;
        let info = StreamInfo::new(name, &config);
        let sample_rate = config.sample_rate;

        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let thread = thread::Builder::new()
            .name("groove-dsp".into())
            .spawn(move || {
                dsp_loop(consumer, &controls, &thread_shutdown, sample_rate, on_frame, on_error);
                log::info!("thread DSP encerrada");
            })
            .map_err(|e| AudioError::Thread(e.to_string()))?;

        Ok(Self { shutdown, thread: Some(thread), stream: Some(stream), info })
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(stream) = self.stream.take() {
            if let Err(e) = stream.pause() {
                log::debug!("pause do stream falhou: {e}");
            }
            drop(stream);
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::error!("thread DSP terminou com panic");
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn dsp_loop(
    mut consumer: Consumer<f32>,
    controls: &SharedControls,
    shutdown: &AtomicBool,
    sample_rate: u32,
    mut on_frame: impl FnMut(&SignalFrame),
    on_error: impl FnOnce(String),
) {
    let mut pipeline = DspPipeline::new(sample_rate);
    pipeline.set_gate_dbfs(controls.gate_dbfs());
    let mut applied_gate = controls.gate_dbfs();
    pipeline.set_a4_hz(controls.reference_hz());
    let mut applied_a4 = controls.reference_hz();
    let mut next_frame = Instant::now() + FRAME_INTERVAL;

    while !shutdown.load(Ordering::Acquire) {
        if controls.has_stream_error() {
            on_error("o stream de audio falhou (device desconectado?)".to_owned());
            return;
        }

        let gate = controls.gate_dbfs();
        if gate != applied_gate {
            pipeline.set_gate_dbfs(gate);
            applied_gate = gate;
        }
        let a4 = controls.reference_hz();
        if a4 != applied_a4 {
            pipeline.set_a4_hz(a4);
            applied_a4 = a4;
        }
        if controls.take_clip_reset() {
            pipeline.reset_clips();
        }

        let available = consumer.slots();
        if available > 0
            && let Ok(chunk) = consumer.read_chunk(available)
        {
            let (first, second) = chunk.as_slices();
            pipeline.feed(first);
            pipeline.feed(second);
            chunk.commit_all();
        }

        let now = Instant::now();
        if now >= next_frame {
            let frame = pipeline.frame(BassMode::from_gain(controls.gain()), controls.dropped());
            on_frame(&frame);
            next_frame += FRAME_INTERVAL;
            if next_frame <= now {
                next_frame = now + FRAME_INTERVAL; // atrasou: nao acumular rajadas
            }
        } else {
            thread::sleep(IDLE_SLEEP.min(next_frame - now));
        }
    }
}

/// Estado global gerenciado pelo Tauri.
pub struct AppState {
    pub controls: Arc<SharedControls>,
    engine: Mutex<Option<EngineHandle>>,
    status: Mutex<EngineStatus>,
    /// Preferencias persistidas (definidas no `setup`, quando o caminho de config e conhecido).
    settings: OnceLock<Arc<SettingsStore>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub fn new() -> Self {
        let controls = Arc::new(SharedControls::new(0));
        Self {
            engine: Mutex::new(None),
            status: Mutex::new(EngineStatus {
                state: EngineState::Starting,
                message: None,
                stream: None,
                mode: BassMode::from_gain(controls.gain()),
                gate_dbfs: controls.gate_dbfs(),
                reference_hz: controls.reference_hz(),
            }),
            controls,
            settings: OnceLock::new(),
        }
    }

    /// Liga o armazenamento de preferencias (so a primeira chamada vale).
    pub fn init_settings(&self, store: Arc<SettingsStore>) {
        if self.settings.set(store).is_err() {
            log::warn!("settings ja inicializado");
        }
    }

    /// Preferencias atuais (defaults se o armazenamento ainda nao foi iniciado).
    pub fn settings(&self) -> Settings {
        self.settings.get().map(|s| s.get()).unwrap_or_default()
    }

    /// Altera e agenda a gravacao das preferencias (no-op sem armazenamento).
    pub fn persist(&self, change: impl FnOnce(&mut Settings)) {
        if let Some(store) = self.settings.get() {
            store.update(change);
        }
    }

    /// Grava preferencias pendentes imediatamente (encerramento do app).
    pub fn flush_settings(&self) {
        if let Some(store) = self.settings.get() {
            store.flush();
        }
    }

    pub fn status(&self) -> EngineStatus {
        let mut status = lock(&self.status).clone();
        status.mode = BassMode::from_gain(self.controls.gain());
        status.gate_dbfs = self.controls.gate_dbfs();
        status.reference_hz = self.controls.reference_hz();
        status
    }

    fn set_status(&self, app: &AppHandle, status: EngineStatus) {
        *lock(&self.status) = status.clone();
        if let Err(e) = app.emit(ENGINE_STATUS_EVENT, &status) {
            log::debug!("falha ao emitir engine-status: {e}");
        }
    }

    /// Encerra o engine atual (se houver).
    pub fn stop(&self) {
        let handle = lock(&self.engine).take();
        if let Some(mut handle) = handle {
            handle.stop();
            log::info!("engine encerrado");
        }
    }

    /// (Re)inicia o engine. Falhas de audio viram status de erro, nunca panic.
    /// `strict`: preferencia de device inexistente vira erro (reinicio pedido pelo usuario).
    pub fn start(self: &Arc<Self>, app: &AppHandle, device_pref: Option<&str>, strict: bool) -> EngineStatus {
        self.stop();
        self.set_status(
            app,
            EngineStatus {
                state: EngineState::Starting,
                message: None,
                stream: None,
                mode: BassMode::from_gain(self.controls.gain()),
                gate_dbfs: self.controls.gate_dbfs(),
                reference_hz: self.controls.reference_hz(),
            },
        );

        let frame_app = app.clone();
        let error_app = app.clone();
        let error_state = Arc::clone(self);
        let result = EngineHandle::start(
            Arc::clone(&self.controls),
            device_pref,
            strict,
            move |frame| {
                if let Err(e) = frame_app.emit(SIGNAL_FRAME_EVENT, frame) {
                    log::debug!("falha ao emitir signal-frame: {e}");
                }
            },
            move |message| {
                log::error!("{message}");
                let mut status = error_state.status();
                status.state = EngineState::Error;
                status.message = Some(message);
                error_state.set_status(&error_app, status);
            },
        );

        let status = match result {
            Ok(handle) => {
                let status = EngineStatus {
                    state: EngineState::Running,
                    message: None,
                    stream: Some(handle.info.clone()),
                    mode: BassMode::from_gain(self.controls.gain()),
                    gate_dbfs: self.controls.gate_dbfs(),
                    reference_hz: self.controls.reference_hz(),
                };
                *lock(&self.engine) = Some(handle);
                status
            }
            Err(e) => {
                log::error!("falha ao iniciar o engine: {e}");
                EngineStatus {
                    state: EngineState::Error,
                    message: Some(e.to_string()),
                    stream: None,
                    mode: BassMode::from_gain(self.controls.gain()),
                    gate_dbfs: self.controls.gate_dbfs(),
                    reference_hz: self.controls.reference_hz(),
                }
            }
        };
        self.set_status(app, status.clone());
        status
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
