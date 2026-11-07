import { ref } from "vue";
import { getWindowState, setMiniMode } from "../api/engine";

// Estado compartilhado (módulo): o backend é a fonte da verdade (ele aplica e persiste).
const mini = ref(false);
const wayland = ref(false);
const error = ref<string | null>(null);
let loaded = false;

/** Modo mini (janela compacta, sempre no topo). */
export function useMiniMode() {
  /** Lê o estado real da janela (o modo salvo já foi aplicado no boot pelo backend). */
  async function load() {
    if (loaded) return;
    loaded = true;
    try {
      const state = await getWindowState();
      mini.value = state.mini_mode;
      wayland.value = state.wayland;
    } catch (e) {
      loaded = false;
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function set(enabled: boolean) {
    error.value = null;
    try {
      const state = await setMiniMode(enabled);
      mini.value = state.mini_mode;
      wayland.value = state.wayland;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  return { mini, wayland, error, load, set, toggle: () => set(!mini.value) };
}
