// Backend simulado para rodar o frontend no navegador (`pnpm dev`) fora do Tauri.

import type {
  BassMode,
  DeviceInfo,
  EngineStatus,
  SignalFrame,
} from "../types/engine";

const NOTES = [
  { note: "B", octave: 0, freq: 30.87 },
  { note: "E", octave: 1, freq: 41.2 },
  { note: "A", octave: 1, freq: 55.0 },
  { note: "D", octave: 2, freq: 73.42 },
  { note: "G", octave: 2, freq: 98.0 },
];

const ACTIVE_ATTENUATION_DB = 12;
const CLIP_HOLD_MS = 1500;

let mode: BassMode = "passive";
const status: EngineStatus = {
  state: "running",
  message: null,
  stream: {
    device_name: "Mock Q-24",
    sample_rate: 192000,
    channels: 2,
    sample_format: "I24",
    target_channel: 0,
    below_target_rate: false,
  },
  mode,
};

export function mockSetBassMode(next: BassMode): void {
  mode = next;
}

export function mockGetStatus(): EngineStatus {
  return { ...status, mode };
}

export function mockListDevices(): DeviceInfo[] {
  return [
    { id: "alsa:mock", name: "Mock Q-24", max_channels: 2, sample_rate_ranges: [[44100, 192000]] },
  ];
}

/** Gera frames sintéticos a ~60 Hz; devolve a função de parada. */
export function startMockFrames(onFrame: (frame: SignalFrame) => void): () => void {
  const start = performance.now();
  let lastOver = -Infinity;
  let hold = -120;
  let holdSince = start;
  const timer = setInterval(() => {
    const t = (performance.now() - start) / 1000;
    // Envelope de "pluck" repetido a cada 3 s; ocasionalmente passa de -3 dBFS.
    const phase = (t % 3) / 3;
    const envelope = Math.exp(-phase * 4);
    const base = -30 + 32 * envelope; // -30 .. +2 dBFS (antes do ganho)
    const offset = mode === "active" ? ACTIVE_ATTENUATION_DB : 0;
    const peak = Math.min(0, base - offset);
    const idx = Math.floor(t / 3) % NOTES.length;
    const n = NOTES[idx]!;
    const playing = envelope > 0.05;
    const over = playing && peak >= -3;
    const now = performance.now();
    if (over) lastOver = now;
    // Hold de 1.5 s e queda de 20 dB/s, como o backend.
    const pk = playing ? peak : -120;
    if (pk >= hold) {
      hold = pk;
      holdSince = now;
    } else if (now - holdSince > 1500) {
      hold = Math.max(-120, hold - 20 * 0.016);
    }
    onFrame({
      peak_dbfs: pk,
      peak_hold_dbfs: hold,
      over_threshold: over,
      clipped_recently: performance.now() - lastOver < CLIP_HOLD_MS,
      pitch: playing
        ? { freq_hz: n.freq, note: n.note, octave: n.octave, cents: Math.sin(t * 2) * 12 }
        : null,
      mode,
      sample_rate: 192000,
      dropped_samples: 0,
    });
  }, 16);
  return () => clearInterval(timer);
}
