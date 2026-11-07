# Step 7: Componente VU meter / LED PEAK

**Status:** pending
**Depends on:** 6

## Goal

Criar o componente Vue `PeakMeter.vue` que mostra o nível de pico em dBFS (barra segmentada estilo LED) e replica o LED "PEAK" da Teyun Q-24, com alerta visual forte quando o sinal cruza -3 dBFS.

## Scope

**In scope:**
- `src/components/PeakMeter.vue` recebendo `peakDbfs`, `peakHoldDbfs`, `overThreshold`/`clippedRecently` como props (dados vindos de `useSignalFrame`).
- Escala de dB não-linear (ex.: -60…0 dBFS com segmentos mais densos perto do topo), marcações -24/-12/-6/-3/0.
- Cores por zona (verde/amarelo/vermelho ou paleta coerente com o mesh gradient — ver passo 9), indicador de peak hold.
- LED "PEAK" separado: aceso vermelho intenso com glow (`box-shadow`/`filter: drop-shadow`) e animação de pulso/flash quando acima de -3 dB; permanece aceso pelo hold definido no passo 5.
- Animações via CSS `transform`/`opacity` (GPU-friendly) — evitar mudar `width/height` a 60 Hz; usar `transform: scaleX/scaleY` ou `clip-path`.
- Acessibilidade: `role="meter"`, `aria-valuenow` etc.; respeitar `prefers-reduced-motion` (sem pulsar, mas ainda alerta forte por cor).

**Out of scope:**
- Afinador e toggle (passo 8).
- Fundo e layout global (passo 9).

## Context

- Mockar frames (passo 6) para desenvolver no navegador com `pnpm dev`.
- Na Q-24, o LED PEAK é um único LED vermelho; o "alerta forte" pode incluir também um destaque de borda/vinheta vermelha na janela (confirmar).

## Open questions / ambiguities

- Orientação do meter (horizontal vs vertical)?
- O alerta deve afetar a UI inteira (ex.: borda/vinheta vermelha piscando) ou só o LED?
- Faixa visível do meter (-60 dB? -48 dB?).

## Acceptance criteria

- [ ] Com frames mockados, o meter acompanha suave a 60 fps sem jank (DevTools Performance sem layout thrash).
- [ ] Valor ≥ -3 dBFS acende o LED PEAK imediatamente; volta a apagar após o hold.
- [ ] `prefers-reduced-motion` desativa a animação de pulso.
- [ ] `vue-tsc` limpo.
