<script setup lang="ts">
// Painel temporário para validar a ponte frontend <-> backend.
import { onMounted, onUnmounted, ref, watch } from "vue";
import { activeListenerCount, isTauri } from "../api/events";
import { useBassMode } from "../composables/useBassMode";
import { useEngineStatus } from "../composables/useEngineStatus";
import { useSignalFrame } from "../composables/useSignalFrame";

const { frame } = useSignalFrame();
const { status, error: statusError, restart } = useEngineStatus();
const { mode, error: modeError, toggle } = useBassMode();

const listeners = ref(0);
const fps = ref(0);
let frames = 0;
let timer = 0;

onMounted(() => {
  timer = window.setInterval(() => {
    listeners.value = activeListenerCount();
    fps.value = frames;
    frames = 0;
  }, 1000);
});
onUnmounted(() => clearInterval(timer));

// Conta quantas publicações do frame ocorreram por segundo.
watch(frame, () => {
  frames += 1;
});
</script>

<template>
  <section class="debug">
    <p>
      origem: <b>{{ isTauri() ? "Tauri" : "mock (navegador)" }}</b> · listeners ativos:
      <b>{{ listeners }}</b> · atualizações/s: <b>{{ fps }}</b>
    </p>
    <p>
      modo: <b>{{ mode }}</b>
      <button @click="toggle">alternar Ativo/Passivo</button>
      <button @click="restart()">reiniciar engine</button>
    </p>
    <p v-if="statusError || modeError" class="err">{{ statusError ?? modeError }}</p>
    <pre>status: {{ JSON.stringify(status, null, 2) }}</pre>
    <pre>frame: {{ JSON.stringify(frame, null, 2) }}</pre>
  </section>
</template>

<style scoped>
.debug {
  font: 13px/1.4 ui-monospace, monospace;
  padding: 12px;
}
pre {
  margin: 8px 0;
  white-space: pre-wrap;
}
button {
  margin-left: 8px;
}
.err {
  color: #f66;
}
</style>
