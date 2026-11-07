// Espelha os payloads Rust (src-tauri/src/pipeline.rs, engine.rs, audio/device.rs).

export const SIGNAL_FRAME_EVENT = "signal-frame";
export const ENGINE_STATUS_EVENT = "engine-status";

export type BassMode = "active" | "passive";

export interface PitchInfo {
  freq_hz: number;
  note: string;
  octave: number;
  /** Desvio da nota mais próxima, em [-50, 50]. */
  cents: number;
}

export interface SignalFrame {
  /** Maior pico do intervalo (dBFS). */
  peak_dbfs: number;
  peak_hold_dbfs: number;
  over_threshold: boolean;
  clipped_recently: boolean;
  pitch: PitchInfo | null;
  /** RMS (~300 ms) em dBFS. */
  rms_dbfs: number;
  /** Eventos de clip (cruzamentos de -3 dBFS) desde o último reset. */
  clip_count: number;
  /** Ruído de fundo em dBFS; null até haver silêncio suficiente. */
  noise_floor_dbfs: number | null;
  /** DC offset em % de full scale. */
  dc_offset: number;
  /** Nível abaixo do gate: a nota fica escondida. */
  gated: boolean;
  /** Referência A4 (Hz) usada pelo backend para nota/cents. */
  reference_hz: number;
  mode: BassMode;
  sample_rate: number;
  dropped_samples: number;
}

export type EngineState = "starting" | "running" | "error" | "stopped";

export interface StreamInfo {
  device_name: string;
  sample_rate: number;
  channels: number;
  sample_format: string;
  target_channel: number;
  below_target_rate: boolean;
}

export interface EngineStatus {
  state: EngineState;
  message: string | null;
  stream: StreamInfo | null;
  mode: BassMode;
  gate_dbfs: number;
  reference_hz: number;
}

/** Faixa válida do gate (mesmos limites do backend). */
export const GATE_MIN_DBFS = -80;
export const GATE_MAX_DBFS = -20;
export const DEFAULT_GATE_DBFS = -50;

/** Faixa válida do A4 (mesmos limites do backend). */
export const A4_MIN_HZ = 432;
export const A4_MAX_HZ = 446;
export const DEFAULT_A4_HZ = 440;

export interface DeviceInfo {
  id: string;
  name: string;
  max_channels: number;
  sample_rate_ranges: [number, number][];
}
