import { onMounted, onUnmounted, ref } from "vue";
import { getEngineStatus, restartEngine } from "../api/engine";
import { onEngineStatus, type Unlisten } from "../api/events";
import type { EngineStatus } from "../types/engine";

/** Status do engine: `listen` primeiro, depois leitura inicial (o evento de partida pode ter sido perdido). */
export function useEngineStatus() {
  const status = ref<EngineStatus | null>(null);
  const error = ref<string | null>(null);
  let unlisten: Unlisten | null = null;
  let disposed = false;
  let gotEvent = false;

  onMounted(async () => {
    const un = await onEngineStatus((s) => {
      gotEvent = true;
      status.value = s;
    });
    if (disposed) {
      un();
      return;
    }
    unlisten = un;
    try {
      const initial = await getEngineStatus();
      // Um evento mais novo vence a leitura inicial.
      if (!gotEvent && !disposed) status.value = initial;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  });

  onUnmounted(() => {
    disposed = true;
    unlisten?.();
    unlisten = null;
  });

  async function restart(device?: string) {
    error.value = null;
    try {
      status.value = await restartEngine(device);
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  return { status, error, restart };
}
