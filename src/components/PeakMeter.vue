<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  METER_MAX_DB,
  METER_MIN_DB,
  METER_TICKS,
  PEAK_ALERT_DBFS,
  dbToPosition,
} from "../utils/meterScale";

const props = defineProps<{
  peakDbfs: number;
  peakHoldDbfs: number;
  overThreshold: boolean;
  clippedRecently: boolean;
}>();

/** Velocidade de queda do nível exibido (ataque instantâneo, release suave). */
const FALL_DB_PER_SEC = 50;

const shown = ref(METER_MIN_DB);
let raf = 0;
let last = 0;

function tick(now: number) {
  const dt = last ? Math.min((now - last) / 1000, 0.1) : 0;
  last = now;
  const target = Math.max(props.peakDbfs, METER_MIN_DB);
  shown.value = Math.max(target, shown.value - FALL_DB_PER_SEC * dt);
  raf = requestAnimationFrame(tick);
}

onMounted(() => {
  raf = requestAnimationFrame(tick);
});
onUnmounted(() => cancelAnimationFrame(raf));

const position = computed(() => dbToPosition(shown.value));
const holdPosition = computed(() => dbToPosition(props.peakHoldDbfs));

// Só `transform`/`opacity` mudam a 60 Hz (sem layout).
const revealStyle = computed(() => ({
  transform: `translateX(${((position.value - 1) * 100).toFixed(2)}%)`,
}));
const brightStyle = computed(() => ({
  transform: `translateX(${((1 - position.value) * 100).toFixed(2)}%)`,
}));
const holdStyle = computed(() => ({
  transform: `translateX(${(holdPosition.value * 100).toFixed(2)}%)`,
  opacity: props.peakHoldDbfs > METER_MIN_DB ? 1 : 0,
}));

const ledOn = computed(() => props.overThreshold || props.clippedRecently);

// Valores textuais/ARIA mudam pouco (inteiro em dB) para não spammar leitores de tela.
const ariaValue = computed(() => Math.round(Math.max(props.peakHoldDbfs, METER_MIN_DB)));
const readout = computed(() =>
  props.peakHoldDbfs <= METER_MIN_DB ? "−∞" : props.peakHoldDbfs.toFixed(1).replace("-", "−"),
);
const ticks = METER_TICKS.map((db) => ({
  db,
  label: db === 0 ? "0" : String(db).replace("-", "−"),
  left: `${(dbToPosition(db) * 100).toFixed(2)}%`,
  alert: db === PEAK_ALERT_DBFS,
}));
</script>

<template>
  <div class="peak-meter" :class="{ clip: ledOn }">
    <div class="vignette" :class="{ on: ledOn }" aria-hidden="true"></div>

    <div class="row">
      <div
        class="meter"
        role="meter"
        aria-label="Nível de pico"
        :aria-valuemin="METER_MIN_DB"
        :aria-valuemax="METER_MAX_DB"
        :aria-valuenow="ariaValue"
        :aria-valuetext="`${ariaValue} dBFS`"
      >
        <div class="cells dim"></div>
        <div class="reveal" :style="revealStyle">
          <div class="cells bright" :style="brightStyle"></div>
        </div>
        <div class="hold" :style="holdStyle"></div>
      </div>

      <div class="led" :class="{ on: ledOn }" role="status">
        <span class="lamp" aria-hidden="true"></span>
        <span class="led-label">PEAK</span>
        <span class="sr-only">{{ ledOn ? "Sinal acima de menos 3 dBFS" : "Sinal normal" }}</span>
      </div>
    </div>

    <div class="scale" aria-hidden="true">
      <span
        v-for="t in ticks"
        :key="t.db"
        class="tick"
        :class="{ alert: t.alert }"
        :style="{ left: t.left }"
      >
        {{ t.label }}
      </span>
    </div>
    <div class="readout" aria-hidden="true">{{ readout }} <small>dBFS</small></div>
  </div>
</template>

<style scoped>
.peak-meter {
  /* Paleta sobrescrevível pelo layout (passo 9). */
  --led-green: #2ee86b;
  --led-yellow: #ffd23f;
  --led-red: #ff2d3d;
  --meter-bg: rgb(255 255 255 / 0.06);
  --meter-height: 28px;
  --cell-count: 40;

  position: relative;
  padding: 12px 16px 8px;
  user-select: none;
}

.row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.meter {
  position: relative;
  flex: 1;
  height: var(--meter-height);
  overflow: hidden;
  border-radius: 4px;
  background: var(--meter-bg);
}

/* Células de LED: gradiente por zona (-12 dB e -3 dB) recortado por máscara. */
.cells {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to right,
    var(--led-green) 0 66%,
    var(--led-yellow) 66% 91%,
    var(--led-red) 91% 100%
  );
  -webkit-mask: linear-gradient(to right, #000 calc(100% - 2px), transparent 0) 0 0 / calc(100% / var(--cell-count)) 100% repeat-x;
  mask: linear-gradient(to right, #000 calc(100% - 2px), transparent 0) 0 0 / calc(100% / var(--cell-count)) 100% repeat-x;
}

.dim {
  opacity: 0.16;
}

.reveal {
  position: absolute;
  inset: 0;
  overflow: hidden;
  will-change: transform;
}

.bright {
  will-change: transform;
}

.hold {
  position: absolute;
  inset: 0;
  will-change: transform, opacity;
}

.hold::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  left: -3px;
  width: 3px;
  border-radius: 1px;
  background: #fff;
  box-shadow: 0 0 6px rgb(255 255 255 / 0.7);
}

.led {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  min-width: 48px;
}

.lamp {
  position: relative;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #3a0d12;
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.1);
}

/* Brilho: camada própria animada só por opacity/transform. */
.lamp::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 35%, #ff9aa2, var(--led-red) 55%, #b0000f);
  box-shadow: 0 0 14px 4px rgb(255 45 61 / 0.9);
  opacity: 0;
  transition: opacity 60ms linear;
}

.led.on .lamp::before {
  opacity: 1;
  animation: pulse 0.5s ease-in-out infinite alternate;
}

.led-label {
  font: 700 11px/1 system-ui, sans-serif;
  letter-spacing: 0.12em;
  color: rgb(255 255 255 / 0.45);
}

.led.on .led-label {
  color: var(--led-red);
}

@keyframes pulse {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0.75;
    transform: scale(1.12);
  }
}

.scale {
  position: relative;
  height: 14px;
  margin: 4px 64px 0 0; /* alinha com .meter (largura do LED + gap) */
}

.tick {
  position: absolute;
  transform: translateX(-50%);
  font: 10px/14px system-ui, sans-serif;
  font-variant-numeric: tabular-nums;
  color: rgb(255 255 255 / 0.5);
}

.tick.alert {
  color: var(--led-red);
  font-weight: 700;
}

.readout {
  margin-top: 2px;
  width: 11ch;
  contain: layout paint;
  font: 600 13px/1.2 ui-monospace, monospace;
  font-variant-numeric: tabular-nums;
  color: rgb(255 255 255 / 0.8);
}

.readout small {
  font-size: 10px;
  opacity: 0.6;
}

/* Vinheta vermelha sutil na janela inteira quando há clip. */
.vignette {
  position: fixed;
  inset: 0;
  z-index: 100;
  pointer-events: none;
  box-shadow: inset 0 0 70px 6px rgb(255 45 61 / 0.55);
  opacity: 0;
  transition: opacity 120ms linear;
}

.vignette.on {
  opacity: 1;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

@media (prefers-reduced-motion: reduce) {
  .led.on .lamp::before {
    animation: none;
  }
  .lamp::before,
  .vignette {
    transition: none;
  }
}
</style>
