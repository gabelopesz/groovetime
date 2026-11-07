<script setup lang="ts">
import BassModeToggle from "./components/BassModeToggle.vue";
import DebugPanel from "./components/DebugPanel.vue";
import PeakMeter from "./components/PeakMeter.vue";
import StatusFooter from "./components/StatusFooter.vue";
import TunerDisplay from "./components/TunerDisplay.vue";
import { useSignalFrame } from "./composables/useSignalFrame";

const { frame } = useSignalFrame();
</script>

<template>
  <main class="app">
    <PeakMeter
      :peak-dbfs="frame?.peak_dbfs ?? -120"
      :peak-hold-dbfs="frame?.peak_hold_dbfs ?? -120"
      :over-threshold="frame?.over_threshold ?? false"
      :clipped-recently="frame?.clipped_recently ?? false"
    />
    <TunerDisplay :pitch="frame?.pitch ?? null" />
    <BassModeToggle :confirmed-mode="frame?.mode ?? null" />
    <StatusFooter />
    <details class="debug">
      <summary>Debug</summary>
      <DebugPanel :frame="frame" />
    </details>
  </main>
</template>

<style>
:root {
  color-scheme: dark;
  font-family: system-ui, sans-serif;
}

body {
  margin: 0;
  background: #111;
  color: #eee;
}

.app {
  min-height: 100vh;
  padding: 8px;
}

.debug {
  margin-top: 16px;
  font-size: 12px;
  color: rgb(255 255 255 / 0.5);
}
</style>
