<script setup lang="ts">
import { computed } from "vue";
import { useEngineStatus } from "../composables/useEngineStatus";

const { status, error, restart } = useEngineStatus();

const info = computed(() => {
  const s = status.value?.stream;
  if (!s) return "";
  return `${s.device_name} · ${(s.sample_rate / 1000).toFixed(s.sample_rate % 1000 ? 1 : 0)} kHz · ${s.sample_format}`;
});
const failed = computed(() => status.value?.state === "error");
</script>

<template>
  <footer class="footer">
    <template v-if="failed">
      <span class="err" role="alert">Áudio indisponível: {{ status?.message }}</span>
      <button type="button" @click="restart()">Tentar novamente</button>
    </template>
    <span v-else-if="status?.state === 'starting'">Iniciando áudio…</span>
    <span v-else>{{ info }}</span>
    <span v-if="status?.stream?.below_target_rate" class="warn">(abaixo de 192 kHz)</span>
    <span v-if="error" class="err" role="alert">{{ error }}</span>
  </footer>
</template>

<style scoped>
.footer {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  align-self: center;
  max-width: 100%;
  padding: 6px 14px;
  border: 1px solid var(--panel-border);
  border-radius: 999px;
  background: var(--panel-bg);
  font: 11px/1.4 var(--font-sans);
  color: var(--text-muted);
}

.err {
  color: #ff6b6b;
}

.warn {
  color: #ffb347;
}

button {
  font: inherit;
  color: inherit;
  background: transparent;
  border: 1px solid currentcolor;
  border-radius: 4px;
  cursor: pointer;
}
</style>
