# Step 9: Mesh gradient CSS + layout final

**Status:** done
**Depends on:** 7, 8

## Goal

Montar o layout final do app e o fundo "mesh gradient" 100% CSS (múltiplos `radial-gradient` sobrepostos + `filter: blur()`) com manchas orgânicas de azul vibrante, laranja e amarelo/bege, garantindo legibilidade e performance com os componentes a 60 fps por cima.

## Scope

**In scope:**
- `src/components/MeshBackground.vue` (ou CSS global): camada `position: fixed; inset: 0; z-index: -1` com vários `radial-gradient(...)` em posições diferentes (azul vibrante ex. `#1f5bff`, laranja `#ff7a1a`, amarelo/bege `#f3d9a4`/`#ffd36b`), `filter: blur(60–120px)` aplicado a pseudo-elementos/blobs, `inset` negativo para evitar bordas borradas aparecendo.
- Opcional: animação lenta (`@keyframes` com `transform` em 30–60 s) desativada em `prefers-reduced-motion`.
- Tokens de design em `:root` (cores, raios, espaçamento, fonte) e aplicação nos componentes dos passos 7–8 (painéis "glass" com `backdrop-filter` se performar bem no WebKitGTK).
- `App.vue` final: layout com afinador em destaque, peak meter + LED PEAK, toggle de modo, status do device; responsivo ao redimensionar a janela; tamanho mínimo de janela em `tauri.conf.json`.
- Remover componente de debug do passo 6.
- Fontes: locais (bundle) ou do sistema — **sem CDN** (CSP restrita).

**Out of scope:**
- Mudanças de lógica Rust.
- Empacotamento (passo 10).

## Context

- WebKitGTK (motor do Tauri no Linux) pode ter custo alto com `filter: blur()` grande e `backdrop-filter` em GPUs integradas; o blur deve ser estático (não re-renderizado a cada frame) — isolar a camada (`will-change: transform` / `contain: strict`) e garantir que os componentes animados a 60 Hz não forcem repaint do fundo.
- Requisito do usuário: o gradiente é 100% CSS — sem imagens, canvas ou WebGL.

## Open questions / ambiguities

- Fundo animado (movimento lento) ou estático?
- Tema escuro com manchas vivas, ou fundo claro/bege predominante? (Afeta contraste do LED PEAK vermelho e do texto.)
- Fonte preferida (ex.: Inter, JetBrains Mono para números)? Precisa ser empacotada localmente.

## Acceptance criteria

- [x] `MeshBackground.vue`: 4 blobs (azul, laranja, amarelo, bege) com `radial-gradient` + `filter: blur(70px)`, só CSS (sem imagem/canvas/WebGL). Verificado por screenshot no app Tauri (480×720, 360×560 e 1100×900).
- [x] (parcial) CPU medido em `pnpm tauri dev` (build **debug**) por 10 s, **com o sinal parado** (mic onboard em silêncio): WebKitWebProcess **10.8%** de 1 core com blobs vs **9.2%** sem blobs (≈ +1.6 pt pelo blur estático); com `backdrop-filter` nos painéis 10.1% (sem diferença mensurável em repouso, mas **não testado com o meter animando** → deixei de fora). O processo `groove-time` (~25%) é o Rust em build debug (DSP a 192 kHz + 60 eventos/s), alheio ao fundo. Não medi fps nem GPU.
- [x] Contraste calculado no pior caso (painel `rgb(9 11 20 / .64)` sobre amarelo `#ffd36b`/bege `#f3d9a4` em intensidade máxima): texto ≈ 6.4–6.6:1 e texto atenuado ≈ 4.7–4.8:1 (sobre laranja/azul é maior). Rodapé também ganhou fundo de painel. Cálculo teórico, não medido por pixel.
- [x] Screenshots em 360×560 (mínimo), 480×720 e 1100×900: layout coluna de até 560 px centralizada, afinador em destaque (`flex: 1`); letra do afinador usa `clamp()`.

## Result

- Criados `src/styles/{tokens,fonts,base}.css` (tokens em `:root`: fontes, cores de texto, manchas, painel, estados de LED/afinador), `src/components/MeshBackground.vue`. `App.vue` final: `MeshBackground` + painéis (afinador hero, meter+PEAK, toggle) + rodapé em pílula. **`DebugPanel.vue` removido.** Componentes dos passos 7/8/footer passaram a usar os tokens (`--font-sans`, `--font-mono`, `--text-muted`, `--led-*`, `--tuner-*`, `--meter-bg`).
- **Decisões do usuário:** fundo **estático**; tema **escuro com manchas vivas**; **Inter + JetBrains Mono empacotadas** (pacotes `@fontsource-variable/*`, só subset latin via `@font-face` próprio em `fonts.css`; `vite.config.ts` com `build.assetsInlineLimit: 0` porque a CSP não permite fontes `data:`). `∞` (U+221E) não está no subset latin e cai na fonte do sistema.
- Fundo isolado: `position: fixed; contain: strict; will-change: transform; z-index: -1`; manchas em unidades `vmin` com cores sólidas no centro (30%) para ficarem vivas.
- Painéis **sem** `backdrop-filter` (WebKitGTK re-borraria o fundo a cada frame do meter); `--panel-bg` ajustado para 0.64 por causa do contraste.
- Janela: 480×720 padrão, mínimo 360×560 (`tauri.conf.json`).
- Pendências: o glifo "–" da letra em "Sem sinal" ainda é um traço largo; CPU/FPS com sinal real e em build release não medidos (passo 14).
