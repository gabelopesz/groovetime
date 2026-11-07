<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

const props = defineProps<{
  /** Desvio em cents (positivo = agudo); `null` = sem sinal. */
  cents: number | null;
  /** Dentro desta tolerância o padrão para. */
  tolerance: number;
}>();

/** Largura de um ciclo (faixa clara + escura), em px. */
const PERIOD = 48;
/** Velocidade por cent de desvio (px/s). Agudo = move para a direita, grave = esquerda. */
const PX_PER_SEC_PER_CENT = 8;
/** Suavização da velocidade (1/s) para não dar saltos quando o desvio oscila. */
const SPEED_SMOOTHING = 12;

const band = ref<HTMLElement | null>(null);
let raf = 0;
let last = 0;
let offset = 0;
let speed = 0;
let lastApplied = Number.NaN;

// A fase é acumulada a cada frame (mudar `animation-duration` causaria saltos);
// só `transform` é escrito, direto no elemento (sem reatividade a 60 Hz).
function tick(now: number) {
  const dt = last ? Math.min((now - last) / 1000, 0.1) : 0;
  last = now;

  const cents = props.cents;
  const target = cents === null || Math.abs(cents) <= props.tolerance ? 0 : cents * PX_PER_SEC_PER_CENT;
  speed += (target - speed) * Math.min(1, dt * SPEED_SMOOTHING);
  if (target === 0 && Math.abs(speed) < 0.5) speed = 0;

  offset = (offset + speed * dt) % PERIOD;
  if (offset < 0) offset += PERIOD;

  const el = band.value;
  if (el && offset !== lastApplied) {
    el.style.transform = `translate3d(${(offset - PERIOD).toFixed(2)}px, 0, 0)`;
    lastApplied = offset;
  }
  raf = requestAnimationFrame(tick);
}

onMounted(() => {
  raf = requestAnimationFrame(tick);
});
onUnmounted(() => cancelAnimationFrame(raf));
</script>

<template>
  <div
    class="strobe"
    :class="{ idle: cents === null }"
    role="img"
    :aria-label="
      cents === null
        ? 'Strobe: sem sinal'
        : Math.abs(cents) <= tolerance
          ? 'Strobe parado: afinado'
          : cents > 0
            ? 'Strobe girando para a direita: agudo'
            : 'Strobe girando para a esquerda: grave'
    "
  >
    <div ref="band" class="band"></div>
    <span class="center" aria-hidden="true"></span>
  </div>
</template>

<style scoped>
.strobe {
  position: relative;
  height: 36px;
  overflow: hidden;
  border-radius: 6px;
  background: var(--meter-bg);
  /* esmaece as bordas, como num strobe de hardware */
  -webkit-mask-image: linear-gradient(to right, transparent, #000 12%, #000 88%, transparent);
  mask-image: linear-gradient(to right, transparent, #000 12%, #000 88%, transparent);
}

.band {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: calc(100% + 96px);
  background: repeating-linear-gradient(
    to right,
    var(--tuner-off) 0 24px,
    rgb(255 255 255 / 0.06) 24px 48px
  );
  will-change: transform;
}

.idle .band {
  opacity: 0.2;
}

.center {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 2px;
  background: rgb(255 255 255 / 0.6);
}
</style>
