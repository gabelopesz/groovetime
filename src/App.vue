<script setup lang="ts">
import { resetClipCount } from "./api/engine";
import BassModeToggle from "./components/BassModeToggle.vue";
import DiagnosticsPanel from "./components/DiagnosticsPanel.vue";
import MeshBackground from "./components/MeshBackground.vue";
import PeakMeter from "./components/PeakMeter.vue";
import StatusFooter from "./components/StatusFooter.vue";
import TunerControls from "./components/TunerControls.vue";
import TunerDisplay from "./components/TunerDisplay.vue";
import { useSignalFrame } from "./composables/useSignalFrame";
import { useTunerSettings } from "./composables/useTunerSettings";

const { frame } = useSignalFrame();
const { tuning, view, referenceHz } = useTunerSettings();

function onResetClips() {
  resetClipCount().catch((e) => console.error("reset_clip_count falhou", e));
}
</script>

<template>
  <MeshBackground />
  <main class="app">
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

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
