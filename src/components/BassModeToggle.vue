<script setup lang="ts">
import { computed } from "vue";
import { useBassMode } from "../composables/useBassMode";
import type { BassMode } from "../types/engine";

const props = defineProps<{
  /** Modo confirmado pelo backend (`SignalFrame.mode`), se já houver frame. */
  confirmedMode?: BassMode | null;
}>();

const { mode, error, toggle } = useBassMode();

const active = computed(() => mode.value === "active");
/** Pedido enviado, mas ainda não confirmado pelo backend. */
const pending = computed(() => !!props.confirmedMode && props.confirmedMode !== mode.value);
</script>

<template>
  <div class="mode">
    <button
      type="button"
      class="switch"
      :class="{ on: active, pending }"
      role="switch"
      :aria-checked="active"
      aria-label="Baixo ativo (atenuação de 12 dB)"
      aria-describedby="mode-help"
      @click="toggle"
    >
      <span class="side" :class="{ current: !active }">Passivo</span>
      <span class="track" aria-hidden="true"><span class="thumb"></span></span>
      <span class="side" :class="{ current: active }">Ativo (−12 dB)</span>
    </button>
    <p id="mode-help" class="help">
      Ativo atenua o sinal em 12 dB para pré-amps ativos não estourarem.
    </p>
    <p v-if="error" class="error" role="alert">Não foi possível alterar o modo: {{ error }}</p>
  </div>
</template>

<style scoped>
.mode {
  padding: 4px 16px 12px;
  text-align: center;
}

.switch {
  display: inline-flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  border: 1px solid rgb(255 255 255 / 0.18);
  border-radius: 999px;
  background: rgb(255 255 255 / 0.05);
  color: inherit;
  font: 600 14px/1 system-ui, sans-serif;
  cursor: pointer;
}

.switch:focus-visible {
  outline: 3px solid #7aa7ff;
  outline-offset: 3px;
}

.side {
  color: rgb(255 255 255 / 0.45);
  transition: color 120ms linear;
}

.side.current {
  color: #fff;
}

.track {
  position: relative;
  width: 44px;
  height: 24px;
  border-radius: 12px;
  background: rgb(255 255 255 / 0.2);
  transition: background-color 120ms linear;
}

.thumb {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fff;
  transition: transform 120ms ease;
}

.on .track {
  background: #3d8bff;
}

.on .thumb {
  transform: translateX(20px);
}

.pending .thumb {
  opacity: 0.6;
}

.help {
  margin: 8px 0 0;
  font: 12px/1.3 system-ui, sans-serif;
  color: rgb(255 255 255 / 0.5);
}

.error {
  margin: 8px 0 0;
  font: 600 13px/1.3 system-ui, sans-serif;
  color: #ff6b6b;
}

@media (prefers-reduced-motion: reduce) {
  .side,
  .track,
  .thumb {
    transition: none;
  }
}
</style>
