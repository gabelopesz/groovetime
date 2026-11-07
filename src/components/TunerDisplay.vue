<script setup lang="ts">
import { computed } from "vue";
import { CHROMATIC, DEFAULT_TUNING, IN_TUNE_CENTS, type Tuning } from "../data/tunings";
import { usePrefersReducedMotion } from "../composables/usePrefersReducedMotion";
import type { PitchInfo } from "../types/engine";
import { DEFAULT_A4_HZ } from "../types/engine";
import { nearestString } from "../utils/tuning";
import StrobeBand from "./StrobeBand.vue";

const props = withDefaults(
  defineProps<{
    pitch: PitchInfo | null;
    /** Nível abaixo do gate (a nota está escondida). */
    gated?: boolean;
    /** Referência A4 (Hz) usada para calcular a corda alvo do preset. */
    referenceHz?: number;
    /** Preset de afinação; sem cordas = cromático (nota/cents do backend). */
    tuning?: Tuning;
    view?: "needle" | "strobe";
    toleranceCents?: number;
  }>(),
  {
    gated: false,
    referenceHz: DEFAULT_A4_HZ,
    tuning: () => DEFAULT_TUNING,
    view: "needle",
    toleranceCents: IN_TUNE_CENTS,
  },
);
const { reduced } = usePrefersReducedMotion();
const showStrobe = computed(() => props.view === "strobe" && !reduced.value);
const strings = computed(() => (props.tuning ?? CHROMATIC).strings);

const MINUS = "−";
const hasSignal = computed(() => props.pitch !== null);

// Com preset: corda mais próxima e cents em relação a ela; sem preset (ou longe demais): nota do backend.
const reading = computed(() => {
  const p = props.pitch;
  if (!p) return null;
  const target = nearestString(p.freq_hz, strings.value, props.referenceHz);
  if (target) {
    const str = strings.value[target.index]!;
    return { note: str.note, octave: str.octave, cents: target.cents, activeIndex: target.index };
  }
  return { note: p.note, octave: p.octave, cents: p.cents, activeIndex: -1 };
});
const cents = computed(() => Math.max(-50, Math.min(50, reading.value?.cents ?? 0)));
const realCents = computed(() => Math.max(-100, Math.min(100, reading.value?.cents ?? 0)));
const inTune = computed(() => hasSignal.value && Math.abs(cents.value) <= props.toleranceCents);

const noteLetter = computed(() => reading.value?.note ?? "–");
const octave = computed(() => (reading.value ? String(reading.value.octave) : ""));
const freq = computed(() => (props.pitch ? props.pitch.freq_hz.toFixed(2) : "—"));

const signed = (v: number) => `${v >= 0 ? "+" : MINUS}${Math.abs(v).toFixed(0)}`;
const status = computed(() => {
  if (!hasSignal.value) return props.gated ? "Abaixo do gate" : "Sem sinal";
  if (inTune.value) return "Afinado";
  return `${realCents.value > 0 ? "Agudo" : "Grave"} ${signed(realCents.value)}¢`;
});

// A agulha só usa `transform` (camada de 100% da largura; 1 cent = 1% do trilho).
const needleStyle = computed(() => ({ transform: `translateX(${cents.value.toFixed(2)}%)` }));

const activeString = computed(() => reading.value?.activeIndex ?? -1);

const ariaCents = computed(() => Math.round(cents.value));
const CENT_MARKS = [-50, -25, 0, 25, 50];
</script>

<template>
  <section class="tuner" :class="{ 'in-tune': inTune, idle: !hasSignal }" aria-label="Afinador">
    <div class="note" aria-live="off">
      <span class="letter">{{ noteLetter }}</span>
      <sub class="octave">{{ octave }}</sub>
    </div>
    <div class="hz">{{ freq }} <small>Hz</small></div>

    <StrobeBand v-if="showStrobe" class="strobe-view" :cents="reading ? reading.cents : null" :tolerance="toleranceCents" />
    <div
      v-else
      class="gauge"
      role="meter"
      aria-label="Desvio de afinação em cents"
      :aria-valuemin="-50"
      :aria-valuemax="50"
      :aria-valuenow="ariaCents"
      :aria-valuetext="status"
    >
      <div class="zone" :style="{ width: `${toleranceCents * 2}%` }"></div>
      <span v-for="m in CENT_MARKS" :key="m" class="mark" :class="{ center: m === 0 }" :style="{ left: `${m + 50}%` }"></span>
      <div class="needle-layer" :style="needleStyle">
        <div class="needle"></div>
      </div>
    </div>
    <div v-if="!showStrobe" class="gauge-scale" aria-hidden="true">
      <span>{{ MINUS }}50¢</span><span>0</span><span>+50¢</span>
    </div>

    <p class="status" role="status">{{ status }}</p>

    <ul v-if="strings.length" class="strings" aria-label="Cordas de referência">
      <li v-for="(s, i) in strings" :key="s.note + s.octave" :class="{ active: i === activeString }">
        <span class="s-name">{{ s.name }}</span>
        <sub>{{ s.octave }}</sub>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.tuner {
  --tuner-ok: #2ee86b;
  --tuner-off: #ffb347;
  --tuner-muted: var(--text-muted);
  padding: 8px 16px 12px;
  text-align: center;
}

.note {
  display: flex;
  justify-content: center;
  align-items: baseline;
  gap: 4px;
  line-height: 1;
}

.letter {
  min-width: 1.2ch;
  font: 800 clamp(72px, min(24vw, 15vh), 128px) / 1 var(--font-sans);
  transition: color 120ms linear;
}

.octave {
  font: 600 28px/1 var(--font-sans);
  color: var(--tuner-muted);
  min-width: 0.8ch;
  text-align: left;
}

.hz {
  font: 600 18px/1.4 var(--font-mono);
  font-variant-numeric: tabular-nums;
  color: var(--tuner-muted);
}

.hz small {
  font-size: 12px;
}

.gauge {
  position: relative;
  height: 36px;
  margin: 12px 0 2px;
  border-radius: 6px;
  background: var(--meter-bg, rgb(255 255 255 / 0.06));
  overflow: hidden;
}

.zone {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  background: rgb(46 232 107 / 0.18);
}

.mark {
  position: absolute;
  top: 55%;
  bottom: 0;
  width: 1px;
  background: rgb(255 255 255 / 0.25);
}

.mark.center {
  top: 0;
  background: var(--text-muted);
}

.needle-layer {
  position: absolute;
  inset: 0;
  transition: transform 90ms linear;
  will-change: transform;
}

.needle {
  position: absolute;
  top: 3px;
  bottom: 3px;
  left: calc(50% - 2px);
  width: 4px;
  border-radius: 2px;
  background: var(--tuner-off);
  box-shadow: 0 0 8px currentColor;
  color: var(--tuner-off);
  transition: background-color 120ms linear;
}

.strobe-view {
  margin: 12px 0 2px;
}

.gauge-scale {
  display: flex;
  justify-content: space-between;
  font: 10px/14px var(--font-sans);
  color: var(--tuner-muted);
}

.status {
  margin: 8px 0 12px;
  font: 700 16px/1.2 var(--font-sans);
  color: var(--tuner-off);
  min-height: 1.2em;
}

.strings {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.strings li {
  min-width: 44px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid rgb(255 255 255 / 0.15);
  font: 700 20px/1 var(--font-sans);
  color: var(--tuner-muted);
  transition: background-color 120ms linear, color 120ms linear;
}

.strings sub {
  font-size: 12px;
}

.strings li.active {
  color: #111;
  background: var(--tuner-off);
  border-color: transparent;
}

.in-tune .letter,
.in-tune .status {
  color: var(--tuner-ok);
}

.in-tune .needle {
  background: var(--tuner-ok);
  color: var(--tuner-ok);
}

.in-tune .strings li.active {
  background: var(--tuner-ok);
}

.idle .needle {
  opacity: 0.25;
}

.idle .letter,
.idle .status {
  color: var(--tuner-muted);
}

@media (prefers-reduced-motion: reduce) {
  .needle-layer,
  .needle,
  .letter,
  .strings li {
    transition: none;
  }
}
</style>
