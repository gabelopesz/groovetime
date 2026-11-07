<script setup lang="ts">
import { computed, onMounted } from "vue";
import { TUNINGS } from "../data/tunings";
import { useTunerSettings } from "../composables/useTunerSettings";
import { usePrefersReducedMotion } from "../composables/usePrefersReducedMotion";
import { A4_MAX_HZ, A4_MIN_HZ, DEFAULT_A4_HZ } from "../types/engine";

const { presetId, view, referenceHz, error, load, setReference, resetReference } = useTunerSettings();
const { reduced } = usePrefersReducedMotion();

onMounted(load);

const atMin = computed(() => referenceHz.value <= A4_MIN_HZ);
const atMax = computed(() => referenceHz.value >= A4_MAX_HZ);
const isDefault = computed(() => referenceHz.value === DEFAULT_A4_HZ);
</script>

<template>
  <div class="controls">
    <label class="field">
      <span>Afinação</span>
      <select v-model="presetId">
        <option v-for="t in TUNINGS" :key="t.id" :value="t.id">{{ t.label }}</option>
      </select>
    </label>

    <div class="field" role="group" aria-label="Referência A4">
      <span>Referência A4</span>
      <div class="stepper">
        <button type="button" aria-label="Diminuir A4 em 1 Hz" :disabled="atMin" @click="setReference(referenceHz - 1)">−</button>
        <output aria-live="polite">{{ referenceHz }} Hz</output>
        <button type="button" aria-label="Aumentar A4 em 1 Hz" :disabled="atMax" @click="setReference(referenceHz + 1)">+</button>
        <button type="button" class="reset" :disabled="isDefault" @click="resetReference()">440</button>
      </div>
    </div>

    <div class="field" role="radiogroup" aria-label="Visualização">
      <span>Visualização</span>
      <div class="segmented">
        <button type="button" role="radio" :aria-checked="view === 'needle'" :class="{ on: view === 'needle' }" @click="view = 'needle'">
          Agulha
        </button>
        <button
          type="button"
          role="radio"
          :aria-checked="view === 'strobe'"
          :class="{ on: view === 'strobe' }"
          :disabled="reduced"
          :title="reduced ? 'Desativado por preferir movimento reduzido' : undefined"
          @click="view = 'strobe'"
        >
          Strobe
        </button>
      </div>
    </div>

    <p v-if="error" class="error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.controls {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
  padding: 0 16px 16px;
  font: 12px/1.3 var(--font-sans);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--text-muted);
}

select,
button {
  font: 600 13px/1.2 var(--font-sans);
  color: var(--text);
}

select {
  padding: 7px 8px;
  border: 1px solid var(--panel-border);
  border-radius: 8px;
  background: rgb(255 255 255 / 0.06);
}

select option {
  color: #111;
}

button {
  padding: 7px 10px;
  border: 1px solid var(--panel-border);
  border-radius: 8px;
  background: rgb(255 255 255 / 0.06);
  cursor: pointer;
}

button:disabled {
  opacity: 0.45;
  cursor: default;
}

select:focus-visible,
button:focus-visible {
  outline: 3px solid var(--focus);
  outline-offset: 2px;
}

.stepper {
  display: flex;
  align-items: stretch;
  gap: 4px;
}

.stepper output {
  flex: 1;
  min-width: 6ch;
  padding: 7px 4px;
  text-align: center;
  font: 600 13px/1.2 var(--font-mono);
  font-variant-numeric: tabular-nums;
  color: var(--text);
}

.segmented {
  display: flex;
}

.segmented button {
  flex: 1;
  border-radius: 0;
}

.segmented button:first-child {
  border-radius: 8px 0 0 8px;
}

.segmented button:last-child {
  border-radius: 0 8px 8px 0;
  border-left: 0;
}

.segmented button.on {
  background: var(--focus);
  color: #0a0d18;
  border-color: var(--focus);
}

.error {
  grid-column: 1 / -1;
  margin: 0;
  color: #ff8b8b;
  font-weight: 600;
}
</style>
