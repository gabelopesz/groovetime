import { computed, ref, watch } from "vue";
import { getSettings, setReferencePitch, setUiPrefs } from "../api/engine";
import { DEFAULT_TUNING, tuningById } from "../data/tunings";
import { A4_MAX_HZ, A4_MIN_HZ, DEFAULT_A4_HZ, type TunerView } from "../types/engine";

export type { TunerView };

// Estado compartilhado (módulo) para que controles e display enxerguem os mesmos valores.
const presetId = ref(DEFAULT_TUNING.id);
const view = ref<TunerView>("needle");
const referenceHz = ref(DEFAULT_A4_HZ);
const error = ref<string | null>(null);
let loaded = false;
let watching = false;

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Preset, modo de visualização e A4 do afinador (hidratados das preferências salvas). */
export function useTunerSettings() {
  const tuning = computed(() => tuningById(presetId.value));

  /** Lê as preferências salvas (uma vez) e passa a persistir mudanças de preset/visualização. */
  async function load() {
    if (loaded) return;
    loaded = true;
    try {
      const saved = await getSettings();
      presetId.value = tuningById(saved.tuning_preset).id;
      view.value = saved.tuner_view === "strobe" ? "strobe" : "needle";
      referenceHz.value = saved.reference_hz;
    } catch (e) {
      loaded = false;
      error.value = message(e);
      return;
    }
    // Só depois de hidratar: evita gravar os defaults por cima do que foi salvo.
    if (!watching) {
      watching = true;
      watch(presetId, (id) => persist({ tuning_preset: id }));
      watch(view, (v) => persist({ tuner_view: v }));
    }
  }

  function persist(prefs: Parameters<typeof setUiPrefs>[0]) {
    setUiPrefs(prefs).catch((e) => {
      error.value = message(e);
    });
  }

  /** Define o A4 (otimista, com rollback se o backend recusar; o backend também persiste). */
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
      error.value = message(e);
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
