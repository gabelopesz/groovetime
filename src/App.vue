<script setup lang="ts">
import DebugPanel from "./components/DebugPanel.vue";
</script>

<template>
  <main class="app">
    <h1>Groove Time</h1>
    <DebugPanel />
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
