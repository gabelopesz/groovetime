<script setup lang="ts">
import { resetClipCount } from "./api/engine";
import { onMounted, onUnmounted } from "vue";
import BassModeToggle from "./components/BassModeToggle.vue";
import DiagnosticsPanel from "./components/DiagnosticsPanel.vue";
import MeshBackground from "./components/MeshBackground.vue";
import MiniView from "./components/MiniView.vue";
import PeakMeter from "./components/PeakMeter.vue";
import StatusFooter from "./components/StatusFooter.vue";
import TunerControls from "./components/TunerControls.vue";
import TunerDisplay from "./components/TunerDisplay.vue";
import { useMiniMode } from "./composables/useMiniMode";
import { useSignalFrame } from "./composables/useSignalFrame";
import { useTunerSettings } from "./composables/useTunerSettings";

const { frame } = useSignalFrame();
const { tuning, view, referenceHz } = useTunerSettings();
const { mini, wayland, error: miniError, load: loadMini, set: setMini, toggle: toggleMini } = useMiniMode();

// Ctrl+M alterna o modo mini (com a janela em foco).
function onKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && !e.altKey && !e.metaKey && e.key.toLowerCase() === "m") {
    e.preventDefault();
    void toggleMini();
  }
}
onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  void loadMini();
});
onUnmounted(() => window.removeEventListener("keydown", onKeydown));

function onResetClips() {
  resetClipCount().catch((e) => console.error("reset_clip_count falhou", e));
}
</script>

<template>
  <MeshBackground />
  <MiniView
    v-if="mini"
    :frame="frame"
    :wayland="wayland"
    @exit="setMini(false)"
    @reset-clips="onResetClips"
  />
  <main v-else class="app">
    <h1 class="sr-only">Groove Time</h1>
    <section class="panel hero">
      <TunerDisplay
        :pitch="frame?.pitch ?? null"
        :gated="frame?.gated ?? false"
        :reference-hz="frame?.reference_hz ?? referenceHz"
        :tuning="tuning"
        :view="view"
      />
      <TunerControls />
    </section>
    <section class="panel">
      <PeakMeter
        :peak-dbfs="frame?.peak_dbfs ?? -120"
        :peak-hold-dbfs="frame?.peak_hold_dbfs ?? -120"
        :over-threshold="frame?.over_threshold ?? false"
        :clipped-recently="frame?.clipped_recently ?? false"
        :rms-dbfs="frame?.rms_dbfs ?? -120"
        :clip-count="frame?.clip_count ?? 0"
        @reset-clips="onResetClips"
      />
    </section>
    <section class="panel">
      <BassModeToggle :confirmed-mode="frame?.mode ?? null" />
    </section>
    <section class="panel">
      <DiagnosticsPanel :frame="frame" />
    </section>
    <div class="mini-row">
      <button type="button" class="mini-btn" @click="setMini(true)">
        Modo mini <kbd>Ctrl+M</kbd>
      </button>
      <p v-if="wayland" class="warn">
        Sessão Wayland: o compositor pode ignorar “sempre no topo”.
      </p>
      <p v-if="miniError" class="warn" role="alert">{{ miniError }}</p>
    </div>
    <StatusFooter />
  </main>
</template>

<style>
.app {
  display: flex;
  flex-direction: column;
  gap: var(--gap);
  width: min(100%, 560px);
  min-height: 100%;
  margin: 0 auto;
  padding: var(--gap);
}

.panel {
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  background: var(--panel-bg);
  /* Sem backdrop-filter: o conteúdo anima a 60 Hz e o WebKitGTK re-borraria o fundo a cada frame. */
  box-shadow: 0 8px 32px rgb(0 0 0 / 0.35);
}

.hero {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.mini-row {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.mini-btn {
  padding: 7px 14px;
  border: 1px solid var(--panel-border);
  border-radius: 999px;
  background: var(--panel-bg);
  color: var(--text);
  font: 600 13px/1.2 var(--font-sans);
  cursor: pointer;
}

.mini-btn:focus-visible {
  outline: 3px solid var(--focus);
  outline-offset: 2px;
}

.mini-btn kbd {
  margin-left: 6px;
  padding: 1px 5px;
  border: 1px solid var(--panel-border);
  border-radius: 4px;
  font: 11px var(--font-mono);
  color: var(--text-muted);
}

.warn {
  margin: 0;
  padding: 4px 10px;
  border-radius: 8px;
  background: var(--panel-bg);
  font: 12px/1.3 var(--font-sans);
  color: var(--tuner-off);
  text-align: center;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
