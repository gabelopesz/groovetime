<script setup lang="ts">
import { useTunerSettings } from "../composables/useTunerSettings";
import type { SignalFrame } from "../types/engine";
import PeakMeter from "./PeakMeter.vue";
import TunerDisplay from "./TunerDisplay.vue";

defineProps<{ frame: SignalFrame | null; wayland: boolean }>();
const emit = defineEmits<{ exit: []; resetClips: [] }>();

const { tuning, view, referenceHz } = useTunerSettings();
</script>

<template>
  <main class="mini">
    <div class="tuner-col">
      <TunerDisplay
        compact
        :pitch="frame?.pitch ?? null"
        :gated="frame?.gated ?? false"
        :reference-hz="frame?.reference_hz ?? referenceHz"
        :tuning="tuning"
        :view="view"
      />
    </div>
    <div class="side-col">
      <PeakMeter
        compact
        :peak-dbfs="frame?.peak_dbfs ?? -120"
        :peak-hold-dbfs="frame?.peak_hold_dbfs ?? -120"
        :over-threshold="frame?.over_threshold ?? false"
        :clipped-recently="frame?.clipped_recently ?? false"
        :rms-dbfs="frame?.rms_dbfs ?? -120"
        :clip-count="frame?.clip_count ?? 0"
        @reset-clips="emit('resetClips')"
      />
      <button
        type="button"
        class="exit"
        aria-label="Sair do modo mini (Ctrl+M)"
        :title="
          wayland
            ? 'Sair do modo mini (Ctrl+M). Em Wayland, “sempre no topo” pode não funcionar.'
            : 'Sair do modo mini (Ctrl+M)'
        "
        @click="emit('exit')"
      >
        ⤢
      </button>
    </div>
  </main>
</template>

<style scoped>
.mini {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 100%;
  padding: 6px;
  overflow: hidden;
}

.tuner-col {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--panel-border);
  border-radius: 14px;
  background: var(--panel-bg);
}

.side-col {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  width: 84px;
  padding: 6px 0;
  border: 1px solid var(--panel-border);
  border-radius: 14px;
  background: var(--panel-bg);
}

.exit {
  width: 32px;
  height: 28px;
  border: 1px solid var(--panel-border);
  border-radius: 8px;
  background: rgb(255 255 255 / 0.06);
  color: var(--text);
  font: 600 15px/1 var(--font-sans);
  cursor: pointer;
}

.exit:focus-visible {
  outline: 3px solid var(--focus);
  outline-offset: 2px;
}
</style>
