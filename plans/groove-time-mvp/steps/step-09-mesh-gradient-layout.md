# Step 9: Mesh gradient CSS + layout final

**Status:** pending
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

- [ ] Fundo implementado só com CSS (`radial-gradient` + `blur`), visual de manchas orgânicas azul/laranja/amarelo-bege.
- [ ] `pnpm tauri dev`: uso de CPU da WebView com sinal ativo razoável (comparar com/sem blur; anotar números) e sem queda perceptível de fps.
- [ ] Contraste de texto principal ≥ 4.5:1 sobre os painéis.
- [ ] Layout funciona do tamanho mínimo ao maximizado.
