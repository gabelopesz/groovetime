import { onMounted, onUnmounted, shallowRef } from "vue";
import { onSignalFrame, type Unlisten } from "../api/events";
import type { SignalFrame } from "../types/engine";

/** Junta dois frames sem perder transientes: mantém o maior pico e o alerta. */
export function mergeFrames(prev: SignalFrame, next: SignalFrame): SignalFrame {
  return {
    ...next,
    peak_dbfs: Math.max(prev.peak_dbfs, next.peak_dbfs),
    over_threshold: prev.over_threshold || next.over_threshold,
  };
}

/**
 * Último `SignalFrame` do backend. Publica no máximo uma vez por frame de
 * renderização (rAF) e usa `shallowRef` para evitar reatividade profunda a 60 Hz.
 */
export function useSignalFrame() {
  const frame = shallowRef<SignalFrame | null>(null);
  let unlisten: Unlisten | null = null;
  let pending: SignalFrame | null = null;
  let raf = 0;
  let disposed = false;

  const publish = () => {
    raf = 0;
    if (pending) {
      frame.value = pending;
      pending = null;
    }
  };

  onMounted(async () => {
    const un = await onSignalFrame((f) => {
      pending = pending ? mergeFrames(pending, f) : f;
      if (!raf) raf = requestAnimationFrame(publish);
    });
    // Desmontou antes de o `listen` resolver: liberar imediatamente.
    if (disposed) un();
    else unlisten = un;
  });

  onUnmounted(() => {
    disposed = true;
    unlisten?.();
    unlisten = null;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    pending = null;
  });

  return { frame };
}
