# Step 7: Componente VU meter / LED PEAK

**Status:** done
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

- [x] (parcial) Por construção só `transform`/`opacity` mudam a 60 Hz (nível via `translateX` em duas camadas, hold via `translateX`, LED via `opacity`/`scale`); release suave por rAF. **Não perfilei no DevTools Performance** — verificado apenas visualmente por screenshot com o mock.
- [x] Screenshot com mock: LED aceso (glow + vinheta) com `over_threshold`/`clipped_recently`. O hold de 1.5 s vem do backend (`clipped_recently`); apagar após o hold não foi observado visualmente.
- [x] `@media (prefers-reduced-motion: reduce)` remove o `animation` do pulso e as transições (o LED e a vinheta continuam por cor). Não testado emulando a preferência.
- [x] `pnpm build` (vue-tsc + vite) limpo.

## Result

- Criados `src/components/PeakMeter.vue` e `src/utils/meterScale.ts` (`dbToPosition`, ticks, `PEAK_ALERT_DBFS`). `App.vue` agora usa `useSignalFrame` uma vez e passa o frame ao `PeakMeter` e ao `DebugPanel` (que virou prop-driven).
- **Decisões do usuário:** meter horizontal; alerta = LED + vinheta vermelha sutil na janela (elemento `position: fixed` dentro do componente — o passo 9 pode movê-lo para o layout); faixa -60…0 dBFS.
- Escala não linear por tabela (ex.: -24→0.40, -12→0.66, -6→0.82, -3→0.91, 0→1); zonas verde até -12, amarelo até -3, vermelho acima (hard stops). 40 células LED por máscara CSS; cores/altura via CSS vars (`--led-green` etc.) para o passo 9 ajustar.
- Nível exibido: ataque instantâneo, queda de 50 dB/s (rAF próprio, pois o frame de silêncio repete -120 e não dispararia `watch`). Leitura numérica = peak hold; `role="meter"` com `aria-valuenow` inteiro (evita ruído em leitores de tela); LED com `role="status"` e texto só para leitores de tela.
- Mock ajustado: `clipped_recently` com hold de 1.5 s e peak hold com queda de 20 dB/s como o backend.
- Verificação visual: Chrome headless contra `pnpm dev` (mock) em 2 instantes. Não verifiquei no app Tauri com sinal real (mic onboard em silêncio) nem a emulação de `prefers-reduced-motion`.
