<script setup lang="ts">
import DebugPanel from "./components/DebugPanel.vue";
import PeakMeter from "./components/PeakMeter.vue";
import { useSignalFrame } from "./composables/useSignalFrame";

const { frame } = useSignalFrame();
</script>

<template>
  <main class="app">
    <h1>Groove Time</h1>
    <PeakMeter
      :peak-dbfs="frame?.peak_dbfs ?? -120"
      :peak-hold-dbfs="frame?.peak_hold_dbfs ?? -120"
      :over-threshold="frame?.over_threshold ?? false"
      :clipped-recently="frame?.clipped_recently ?? false"
    />
    <DebugPanel :frame="frame" />
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

h1 {
  margin: 4px 12px;
  font-size: 18px;
}
</style>
