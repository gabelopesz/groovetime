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
let gate = -50;
let clipCount = 0;
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
  gate_dbfs: gate,
};

/** `?failmode` na URL simula falha do command (para testar o rollback). */
export function mockSetBassMode(next: BassMode): void {
  if (typeof location !== "undefined" && new URLSearchParams(location.search).has("failmode")) {
    throw new Error("falha simulada (mock)");
  }
  mode = next;
}

export function mockGetStatus(): EngineStatus {
  return { ...status, mode, gate_dbfs: gate };
}

export function mockSetGate(value: number): void {
  if (!Number.isFinite(value) || value < -80 || value > -20) {
    throw new Error("valor invalido: gate deve estar entre -80 e -20 dBFS");
  }
  gate = value;
}

export function mockResetClips(): void {
  clipCount = 0;
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
  let wasOver = false;
  let silentSince = performance.now();
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
    if (over && !wasOver) clipCount += 1;
    wasOver = over;
    if (playing) silentSince = now;
    // Hold de 1.5 s e queda de 20 dB/s, como o backend.
    const pk = playing ? peak : -120;
    const rms = playing ? pk - 7 : -120;
    const gated = rms < gate;
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
      pitch: playing && !gated
        ? { freq_hz: n.freq, note: n.note, octave: n.octave, cents: Math.sin(t * 2) * 12 }
        : null,
      rms_dbfs: rms,
      clip_count: clipCount,
      noise_floor_dbfs: now - silentSince > 2000 ? -72.4 : null,
      dc_offset: 0.4,
      gated,
      mode,
      sample_rate: 192000,
      dropped_samples: 0,
    });
  }, 16);
  return () => clearInterval(timer);
}
