import { computed, ref } from "vue";
import { getEngineStatus, setReferencePitch } from "../api/engine";
import { DEFAULT_TUNING, tuningById } from "../data/tunings";
import { A4_MAX_HZ, A4_MIN_HZ, DEFAULT_A4_HZ } from "../types/engine";

export type TunerView = "needle" | "strobe";

// Estado compartilhado (módulo) para que controles e display enxerguem os mesmos valores.
const presetId = ref(DEFAULT_TUNING.id);
const view = ref<TunerView>("needle");
const referenceHz = ref(DEFAULT_A4_HZ);
const error = ref<string | null>(null);
let loaded = false;

/** Preset, modo de visualização e A4 do afinador. */
export function useTunerSettings() {
  const tuning = computed(() => tuningById(presetId.value));

  /** Lê o A4 atual do backend (uma vez). */
  async function load() {
    if (loaded) return;
    loaded = true;
    try {
      referenceHz.value = (await getEngineStatus()).reference_hz;
    } catch (e) {
      loaded = false;
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  /** Define o A4 (otimista, com rollback se o backend recusar). */
  async function setReference(hz: number) {
    const next = Math.min(A4_MAX_HZ, Math.max(A4_MIN_HZ, Math.round(hz)));
    const previous = referenceHz.value;
    if (next === previous) return;
    referenceHz.value = next;
    error.value = null;
    try {
      await setReferencePitch(next);
    } catch (e) {
      referenceHz.value = previous;
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  return {
    presetId,
    tuning,
    view,
    referenceHz,
    error,
    load,
    setReference,
    resetReference: () => setReference(DEFAULT_A4_HZ),
  };
}
