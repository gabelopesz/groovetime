<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getEngineStatus, setGateDbfs } from "../api/engine";
import {
  DEFAULT_GATE_DBFS,
  GATE_MAX_DBFS,
  GATE_MIN_DBFS,
  type SignalFrame,
} from "../types/engine";

const props = defineProps<{ frame: SignalFrame | null }>();

const NOISE_WARN_DBFS = -60;
/** Abaixo disso o ruído é praticamente zero digital: provavelmente não há sinal na entrada. */
const NOISE_DEAD_DBFS = -110;
const DC_WARN_PERCENT = 1;
const MINUS = "−";

const gate = ref(DEFAULT_GATE_DBFS);
const gateError = ref<string | null>(null);
let appliedGate = DEFAULT_GATE_DBFS;

onMounted(async () => {
  try {
    appliedGate = gate.value = (await getEngineStatus()).gate_dbfs;
  } catch {
    /* mantém o default */
  }
});

// Só envia ao backend ao soltar o slider (evento `change`).
async function commitGate() {
  gateError.value = null;
  try {
    await setGateDbfs(gate.value);
    appliedGate = gate.value;
  } catch (e) {
    gate.value = appliedGate;
    gateError.value = e instanceof Error ? e.message : String(e);
  }
}

const fmt = (v: number, digits = 1) => v.toFixed(digits).replace("-", MINUS);

const noise = computed(() => props.frame?.noise_floor_dbfs ?? null);
const dc = computed(() => props.frame?.dc_offset ?? 0);

const noiseText = computed(() => (noise.value === null ? "medindo…" : `${fmt(noise.value)} dBFS`));
const dcText = computed(() => `${dc.value >= 0 ? "+" : MINUS}${Math.abs(dc.value).toFixed(2)} %`);

const hints = computed(() => {
  const out: string[] = [];
  if (noise.value === null) {
    out.push("Deixe as cordas em silêncio por alguns segundos para medir o ruído de fundo.");
  } else if (noise.value <= NOISE_DEAD_DBFS) {
    out.push("Silêncio digital: confira se a entrada certa está ativa e o cabo conectado.");
  } else if (noise.value > NOISE_WARN_DBFS) {
    out.push("Ruído alto: verifique cabo, bateria e aterramento.");
  }
  if (Math.abs(dc.value) > DC_WARN_PERCENT) {
    out.push("DC offset alto: possível problema no captador ou no pré-amp.");
  }
  if (out.length === 0) out.push("Sinal limpo.");
  return out;
});
const noiseBad = computed(() => noise.value !== null && noise.value > NOISE_WARN_DBFS);
const dcBad = computed(() => Math.abs(dc.value) > DC_WARN_PERCENT);
</script>

<template>
  <details class="diag">
    <summary>Diagnóstico</summary>

    <dl class="stats">
      <div :class="{ bad: noiseBad }">
        <dt>Ruído de fundo</dt>
        <dd>{{ noiseText }}</dd>
      </div>
      <div :class="{ bad: dcBad }">
        <dt>DC offset</dt>
        <dd>{{ dcText }}</dd>
      </div>
    </dl>
    <ul class="hints">
      <li v-for="h in hints" :key="h">{{ h }}</li>
    </ul>

    <div class="gate">
      <label for="gate-slider">
        Gate: <b>{{ fmt(gate, 0) }} dBFS</b>
      </label>
      <input
        id="gate-slider"
        v-model.number="gate"
        type="range"
        :min="GATE_MIN_DBFS"
        :max="GATE_MAX_DBFS"
        step="1"
        :aria-valuetext="`${fmt(gate, 0)} dBFS`"
        aria-describedby="gate-help"
        @change="commitGate"
      />
      <p id="gate-help" class="help">Abaixo desse nível a nota é escondida (o medidor continua).</p>
      <p v-if="gateError" class="error" role="alert">{{ gateError }}</p>
    </div>
  </details>
</template>

<style scoped>
.diag {
  padding: var(--panel-padding);
  font: 13px/1.4 var(--font-sans);
}

summary {
  cursor: pointer;
  font-weight: 600;
  color: var(--text);
}

summary:focus-visible,
input:focus-visible {
  outline: 3px solid var(--focus);
  outline-offset: 3px;
}

.stats {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin: 12px 0 8px;
}

.stats div {
  padding: 8px 10px;
  border: 1px solid var(--panel-border);
  border-radius: 10px;
}

.stats dt {
  font-size: 11px;
  color: var(--text-muted);
}

.stats dd {
  margin: 2px 0 0;
  font: 600 15px/1.2 var(--font-mono);
  font-variant-numeric: tabular-nums;
}

.stats .bad {
  border-color: var(--tuner-off);
}

.stats .bad dd {
  color: var(--tuner-off);
}

.hints {
  margin: 0 0 12px;
  padding-left: 18px;
  color: var(--text-muted);
}

.gate label {
  display: block;
  margin-bottom: 4px;
}

.gate input {
  width: 100%;
}

.help {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}

.error {
  margin: 4px 0 0;
  color: #ff8b8b;
  font-weight: 600;
}
</style>
