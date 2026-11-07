// Wrappers tipados dos commands Tauri (com fallback para o mock no navegador).

import { invoke } from "@tauri-apps/api/core";
import {
  mockGetStatus,
  mockListDevices,
  mockResetClips,
  mockSetBassMode,
  mockSetGate,
} from "../mock/engine";
import type { BassMode, DeviceInfo, EngineStatus } from "../types/engine";
import { isTauri } from "./events";

/** Erro de um command (o backend serializa `CommandError` como string). */
export class CommandError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CommandError";
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw new CommandError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
  }
}

export async function setBassMode(mode: BassMode): Promise<void> {
  if (!isTauri()) return mockSetBassMode(mode);
  await call<void>("set_bass_mode", { mode });
}

export async function resetClipCount(): Promise<void> {
  if (!isTauri()) return mockResetClips();
  await call<void>("reset_clip_count");
}

/** Gate em dBFS (-80…-20); fora da faixa o backend devolve `CommandError`. */
export async function setGateDbfs(value: number): Promise<void> {
  if (!isTauri()) return mockSetGate(value);
  await call<void>("set_gate_dbfs", { value });
}

export async function getEngineStatus(): Promise<EngineStatus> {
  if (!isTauri()) return mockGetStatus();
  return call<EngineStatus>("get_engine_status");
}

export async function listInputDevices(): Promise<DeviceInfo[]> {
  if (!isTauri()) return mockListDevices();
  return call<DeviceInfo[]>("list_input_devices");
}

export async function restartEngine(device?: string): Promise<EngineStatus> {
  if (!isTauri()) return mockGetStatus();
  return call<EngineStatus>("restart_engine", { device: device ?? null });
}
