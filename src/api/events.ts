// Assinatura de eventos do backend (Tauri) ou do mock (navegador).

import { listen } from "@tauri-apps/api/event";
import { startMockFrames } from "../mock/engine";
import {
  ENGINE_STATUS_EVENT,
  SIGNAL_FRAME_EVENT,
  type EngineStatus,
  type SignalFrame,
} from "../types/engine";

export type Unlisten = () => void;

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Quantidade de listeners ativos (útil para checar vazamentos no painel de debug). */
let active = 0;
export function activeListenerCount(): number {
  return active;
}

function tracked(unlisten: Unlisten): Unlisten {
  active += 1;
  let done = false;
  return () => {
    if (done) return;
    done = true;
    active -= 1;
    unlisten();
  };
}

export async function onSignalFrame(cb: (frame: SignalFrame) => void): Promise<Unlisten> {
  if (!isTauri()) return tracked(startMockFrames(cb));
  const unlisten = await listen<SignalFrame>(SIGNAL_FRAME_EVENT, (e) => cb(e.payload));
  return tracked(unlisten);
}

export async function onEngineStatus(cb: (status: EngineStatus) => void): Promise<Unlisten> {
  // O mock não emite mudanças de status; o status inicial vem de getEngineStatus.
  if (!isTauri()) return tracked(() => {});
  const unlisten = await listen<EngineStatus>(ENGINE_STATUS_EVENT, (e) => cb(e.payload));
  return tracked(unlisten);
}
