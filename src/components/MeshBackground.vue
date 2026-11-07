<script setup lang="ts"></script>

<template>
  <!-- Fundo 100% CSS: blobs com radial-gradient + blur estático, em camada isolada. -->
  <div class="mesh" aria-hidden="true">
    <span class="blob blue"></span>
    <span class="blob orange"></span>
    <span class="blob yellow"></span>
    <span class="blob beige"></span>
  </div>
</template>

<style scoped>
.mesh {
  position: fixed;
  inset: 0;
  z-index: -1;
  overflow: hidden;
  background: var(--bg-base);
  /* Camada própria e isolada: o que anima por cima não repinta o fundo. */
  contain: strict;
  will-change: transform;
  pointer-events: none;
}

.blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(var(--mesh-blur, 70px));
  opacity: 1;
}

.blue {
  width: 130vmin;
  height: 120vmin;
  top: -35vmin;
  left: -40vmin;
  background: radial-gradient(closest-side at 50% 50%, var(--blob-blue) 30%, transparent 75%);
}

.orange {
  width: 120vmin;
  height: 110vmin;
  right: -45vmin;
  top: 22%;
  background: radial-gradient(closest-side at 50% 50%, var(--blob-orange) 30%, transparent 75%);
}

.yellow {
  width: 110vmin;
  height: 90vmin;
  right: -15vmin;
  bottom: -30vmin;
  background: radial-gradient(closest-side at 50% 50%, var(--blob-yellow) 30%, transparent 72%);
  opacity: 0.85;
}

.beige {
  width: 100vmin;
  height: 80vmin;
  left: -35vmin;
  bottom: 2%;
  background: radial-gradient(closest-side at 50% 50%, var(--blob-beige) 30%, transparent 72%);
  opacity: 0.6;
}
</style>
