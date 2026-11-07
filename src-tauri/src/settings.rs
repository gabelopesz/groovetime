//! Preferencias persistidas em JSON (escritas pelo Rust, sem acesso do frontend ao filesystem).
//!
//! - Carga tolerante: campos ausentes usam o default; arquivo ilegivel vira `.bak`.
//! - Escrita atomica (`.tmp` + `rename`), com debounce, fora da thread DSP e da callback RT.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::audio::{A4_MAX_HZ, A4_MIN_HZ, DEFAULT_A4_HZ, DEFAULT_GATE_DBFS, GATE_MAX_DBFS, GATE_MIN_DBFS};
use crate::pipeline::BassMode;

pub const SETTINGS_VERSION: u32 = 1;
pub const DEFAULT_PRESET: &str = "bass-4";
/// Espera sem novas alteracoes antes de gravar (limita a ~2 gravacoes/s).
pub const WRITE_DEBOUNCE: Duration = Duration::from_millis(500);
const MAX_PRESET_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TunerView {
    #[default]
    Needle,
    Strobe,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub bass_mode: BassMode,
    pub reference_hz: f32,
    pub tuning_preset: String,
    pub tuner_view: TunerView,
    pub gate_dbfs: f32,
    /// Preferencia de device (substring do nome); `None` = automatico.
    pub device_name: Option<String>,
    pub mini_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            bass_mode: BassMode::Passive,
            reference_hz: DEFAULT_A4_HZ,
            tuning_preset: DEFAULT_PRESET.to_owned(),
            tuner_view: TunerView::Needle,
            gate_dbfs: DEFAULT_GATE_DBFS,
            device_name: None,
            mini_mode: false,
        }
    }
}

/// Aceita so ids simples (letras, digitos, `-`, `_`).
pub fn is_valid_preset_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_PRESET_LEN
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Settings {
    /// Corrige valores fora da faixa (mesmas regras dos commands) e normaliza a versao.
    pub fn sanitized(mut self) -> Self {
        if !(self.reference_hz.is_finite() && (A4_MIN_HZ..=A4_MAX_HZ).contains(&self.reference_hz)) {
            self.reference_hz = DEFAULT_A4_HZ;
        }
        if !(self.gate_dbfs.is_finite() && (GATE_MIN_DBFS..=GATE_MAX_DBFS).contains(&self.gate_dbfs)) {
            self.gate_dbfs = DEFAULT_GATE_DBFS;
        }
        if !is_valid_preset_id(&self.tuning_preset) {
            self.tuning_preset = DEFAULT_PRESET.to_owned();
        }
        if self.device_name.as_deref().is_some_and(|d| d.trim().is_empty() || d.len() > 256) {
            self.device_name = None;
        }
        self.version = SETTINGS_VERSION;
        self
    }

    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Settings>(json).map(Settings::sanitized)
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

fn bak_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".bak");
    path.with_file_name(name)
}

/// Grava de forma atomica: `settings.json.tmp` (com fsync) + `rename`.
pub fn write_atomic(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_path(path);
    let bytes = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)
}

/// Le o arquivo. Ausente → defaults; ilegivel/corrompido → log, move para `.bak` e defaults.
pub fn load_or_default(path: &Path) -> Settings {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Settings::default(),
        Err(e) => {
            log::error!("nao foi possivel ler {}: {e}", path.display());
            quarantine(path);
            return Settings::default();
        }
    };
    match Settings::parse(&text) {
        Ok(s) => s,
        Err(e) => {
            log::error!("{} corrompido ({e}); usando defaults", path.display());
            quarantine(path);
            Settings::default()
        }
    }
}

fn quarantine(path: &Path) {
    let bak = bak_path(path);
    if let Err(e) = fs::rename(path, &bak) {
        log::error!("nao foi possivel mover para {}: {e}", bak.display());
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Preferencias em memoria + gravacao com debounce.
pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
    dirty: AtomicBool,
    tx: Sender<()>,
    writes: AtomicUsize,
}

impl SettingsStore {
    /// Carrega `path` (tolerante a erros) e inicia a thread de gravacao.
    pub fn open(path: PathBuf) -> Arc<Self> {
        Self::open_with_debounce(path, WRITE_DEBOUNCE)
    }

    pub fn open_with_debounce(path: PathBuf, debounce: Duration) -> Arc<Self> {
        let settings = load_or_default(&path);
        let (tx, rx) = mpsc::channel();
        let store = Arc::new(Self {
            path,
            current: Mutex::new(settings),
            dirty: AtomicBool::new(false),
            tx,
            writes: AtomicUsize::new(0),
        });
        let weak = Arc::downgrade(&store);
        if let Err(e) = thread::Builder::new()
            .name("groove-settings".into())
            .spawn(move || writer_loop(rx, weak, debounce))
        {
            log::error!("falha ao criar a thread de gravacao de settings: {e}");
        }
        store
    }

    pub fn get(&self) -> Settings {
        lock(&self.current).clone()
    }

    /// Aplica a alteracao e agenda a gravacao (debounce).
    pub fn update(&self, change: impl FnOnce(&mut Settings)) {
        {
            let mut current = lock(&self.current);
            change(&mut current);
        }
        self.dirty.store(true, Ordering::Release);
        // Sem receptor (thread nao subiu): o flush no encerramento ainda grava.
        let _ = self.tx.send(());
    }

    /// Grava agora, se houver alteracoes pendentes.
    pub fn flush(&self) {
        if !self.dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let snapshot = self.get();
        match write_atomic(&self.path, &snapshot) {
            Ok(()) => {
                self.writes.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                log::error!("falha ao gravar {}: {e}", self.path.display());
                self.dirty.store(true, Ordering::Release);
            }
        }
    }

    /// Numero de gravacoes feitas (para testes/diagnostico).
    pub fn write_count(&self) -> usize {
        self.writes.load(Ordering::Relaxed)
    }
}

fn writer_loop(rx: Receiver<()>, store: Weak<SettingsStore>, debounce: Duration) {
    while rx.recv().is_ok() {
        thread::sleep(debounce);
        while rx.try_recv().is_ok() {}
        match store.upgrade() {
            Some(s) => s.flush(),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("groove-settings-{tag}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip() {
        let s = Settings {
            bass_mode: BassMode::Active,
            reference_hz: 442.0,
            tuning_preset: "drop-d".into(),
            tuner_view: TunerView::Strobe,
            gate_dbfs: -42.0,
            device_name: Some("Q-24".into()),
            mini_mode: true,
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(Settings::parse(&json).unwrap(), s);
    }

    #[test]
    fn partial_json_uses_defaults_and_ignores_unknown_fields() {
        let s = Settings::parse(r#"{ "reference_hz": 435.0, "campo_futuro": 1 }"#).unwrap();
        assert_eq!(s.reference_hz, 435.0);
        assert_eq!(s.gate_dbfs, DEFAULT_GATE_DBFS);
        assert_eq!(s.bass_mode, BassMode::Passive);
        assert_eq!(s.tuning_preset, DEFAULT_PRESET);
        assert_eq!(Settings::parse("{}").unwrap(), Settings::default());
    }

    #[test]
    fn out_of_range_values_are_sanitized() {
        let s = Settings::parse(
            r#"{ "reference_hz": 500.0, "gate_dbfs": -200.0, "tuning_preset": "", "device_name": "  " }"#,
        )
        .unwrap();
        assert_eq!(s.reference_hz, DEFAULT_A4_HZ);
        assert_eq!(s.gate_dbfs, DEFAULT_GATE_DBFS);
        assert_eq!(s.tuning_preset, DEFAULT_PRESET);
        assert_eq!(s.device_name, None);
        let s = Settings::parse(r#"{ "tuning_preset": "../../etc" }"#).unwrap();
        assert_eq!(s.tuning_preset, DEFAULT_PRESET);
    }

    #[test]
    fn invalid_enum_is_an_error() {
        assert!(Settings::parse(r#"{ "tuner_view": "outro" }"#).is_err());
        assert!(Settings::parse("nao e json").is_err());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = temp_dir("missing");
        assert_eq!(load_or_default(&dir.join("settings.json")), Settings::default());
    }

    #[test]
    fn corrupted_file_is_moved_to_bak_and_defaults_are_used() {
        let dir = temp_dir("corrupt");
        let path = dir.join("settings.json");
        fs::write(&path, "{ isto nao e json").unwrap();
        assert_eq!(load_or_default(&path), Settings::default());
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(dir.join("settings.json.bak")).unwrap(), "{ isto nao e json");
    }

    #[test]
    fn atomic_write_roundtrip_leaves_no_tmp() {
        let dir = temp_dir("atomic");
        let path = dir.join("sub").join("settings.json");
        let s = Settings { reference_hz: 438.0, ..Settings::default() };
        write_atomic(&path, &s).unwrap();
        assert_eq!(load_or_default(&path), s);
        assert!(!dir.join("sub").join("settings.json.tmp").exists());
    }

    #[test]
    fn debounce_limits_writes_during_continuous_updates() {
        let dir = temp_dir("debounce");
        let path = dir.join("settings.json");
        let store = SettingsStore::open_with_debounce(path.clone(), Duration::from_millis(200));
        // ~600 ms de "arrastar slider": 60 alteracoes a cada 10 ms.
        for i in 0..60 {
            store.update(|s| s.gate_dbfs = -80.0 + (i % 50) as f32);
            thread::sleep(Duration::from_millis(10));
        }
        thread::sleep(Duration::from_millis(500));
        let writes = store.write_count();
        assert!((1..=4).contains(&writes), "writes={writes}");
        // O ultimo valor chegou ao disco.
        assert_eq!(load_or_default(&path).gate_dbfs, store.get().gate_dbfs);
    }

    #[test]
    fn flush_writes_pending_changes_immediately() {
        let dir = temp_dir("flush");
        let path = dir.join("settings.json");
        let store = SettingsStore::open_with_debounce(path.clone(), Duration::from_secs(60));
        store.update(|s| s.mini_mode = true);
        assert!(!path.exists());
        store.flush();
        assert!(load_or_default(&path).mini_mode);
        store.flush(); // sem pendencias: nao regrava
        assert_eq!(store.write_count(), 1);
    }
}
