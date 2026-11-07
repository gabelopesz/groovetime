import { onMounted, ref } from "vue";
import { getEngineStatus, setBassMode } from "../api/engine";
import type { BassMode } from "../types/engine";

/** Modo Ativo/Passivo com estado otimista e rollback se o invoke falhar. */
export function useBassMode() {
  const mode = ref<BassMode>("passive");
  const error = ref<string | null>(null);
  /** `true` se o app abriu em modo Ativo (restaurado das preferências). */
  const restoredActive = ref(false);

  onMounted(async () => {
    try {
      mode.value = (await getEngineStatus()).mode;
      restoredActive.value = mode.value === "active";
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  });

  async function set(next: BassMode) {
    const previous = mode.value;
    if (next === previous) return;
    mode.value = next;
    error.value = null;
    restoredActive.value = false;
    try {
      await setBassMode(next);
    } catch (e) {
      mode.value = previous;
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  const toggle = () => set(mode.value === "active" ? "passive" : "active");

  return { mode, error, restoredActive, set, toggle };
}
