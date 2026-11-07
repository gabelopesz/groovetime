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
}

export interface DeviceInfo {
  id: string;
  name: string;
  max_channels: number;
  sample_rate_ranges: [number, number][];
}
