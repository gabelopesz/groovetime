# Step 8: Afinador + toggle Ativo/Passivo

**Status:** done
**Depends on:** 6

## Goal

Criar o display do afinador (nota, oitava, frequência, desvio em cents com agulha/indicador) e o botão de modo Ativo/Passivo que chama o backend para aplicar/remover a atenuação de -12 dB.

## Scope

**In scope:**
- `src/components/TunerDisplay.vue`: nota grande, oitava, Hz com 1–2 casas, indicador de cents (-50…+50) com zona "afinado" (ex.: ±3 cents), estado "sem sinal". Movimento da agulha via `transform: rotate/translateX` com transição curta.
- Referência das cordas do baixo (padrão 5 cordas B0 E1 A1 D2 G2) destacando a corda mais próxima. Modelar as cordas como dados (`src/data/tunings.ts`, array de `{ name, note, octave }`), pois o passo 11 adiciona presets de afinação sobre essa estrutura.
- `src/components/BassModeToggle.vue`: toggle acessível (`role="switch"`, `aria-checked`), labels "Passivo" / "Ativo (-12 dB)", usa `useBassMode` (estado otimista + rollback em erro), feedback visual de erro.
- Exibir rate efetiva/device em rodapé discreto (de `useEngineStatus`) — opcional aqui ou no passo 9.

**Out of scope:**
- Peak meter (passo 7), fundo/layout final (passo 9).
- Seleção de afinação alternativa (fora do plano).

## Context

- `PitchInfo` vem com `note`, `octave`, `cents`, `freq_hz` (passo 5). Estabilização já é feita no Rust; a UI só suaviza visualmente.

## Open questions / ambiguities

- Mostrar cordas para 4, 5 ou 6 cordas? Auto (todas) ou configurável?
- Tolerância de "afinado" (±2, ±3, ±5 cents)?
- Nomenclatura de notas: sustenidos (C#) ou bemóis (Db)? Notação brasileira (Dó, Ré…) ou anglo (C, D…)?
- Atalho de teclado para o toggle?

## Acceptance criteria

- [x] Com mock (Chrome real, screenshots): nota/oitava/Hz/cents atualizam; |cents| ≤ 3 → "Afinado" (verde, corda destacada); fora → "Grave −10¢"/"Agudo" (laranja); sem pitch → "Sem sinal".
- [x] No mock o toggle altera o modo e o frame confirma (`pending` se divergir); falha simulada (`?failmode`) faz rollback para "Passivo" com mensagem de erro (`role="alert"`). Contra o backend Tauri real essa cadeia já tinha sido validada no passo 6 (`invoke` → `frame.mode`), mas não repeti com este componente.
- [x] Teclado: Tab foca (anel de foco visível) e Espaço alterna (verificado). Semântica: `button role="switch"` + `aria-checked`, `aria-label`, `aria-describedby`. **Não testado com leitor de tela real.**
- [x] `pnpm build` (vue-tsc + vite) limpo.

## Result

- Criados `src/data/tunings.ts` (`Tuning`, `BASS_4`, `DEFAULT_TUNING`, `IN_TUNE_CENTS`), `src/components/{TunerDisplay,BassModeToggle,StatusFooter}.vue`. `App.vue` empilha PeakMeter + Tuner + Toggle + Footer, com o `DebugPanel` dentro de `<details>` (o layout final é do passo 9).
- **Decisões do usuário:** 4 cordas (E1 A1 D2 G2); tolerância ±3 cents; notação anglo com sustenidos; sem atalho global (toggle focável, Espaço/Enter nativos de `button`).
- Corda de referência destacada só quando nota+oitava detectadas coincidem exatamente com uma corda (ex.: B0 do mock mostra a nota mas não destaca nenhuma corda). O passo 11 pode estender com presets (5/6 cordas) trocando `strings`.
- Agulha: camada de 100% da largura com `translateX(cents %)` (só `transform`, transição de 90 ms; sem transição com `prefers-reduced-motion`). `role="meter"` com `aria-valuetext`.
- `BassModeToggle` usa `useBassMode` (otimista + rollback) e recebe `confirmedMode` (do frame) para sinalizar pedido pendente. `StatusFooter` mostra device · kHz · formato, aviso "abaixo de 192 kHz", e em erro do engine mostra mensagem + "Tentar novamente" (`restart_engine`).
- Mock: `?failmode` na URL simula falha do `set_bass_mode`.
- Observação visual: em "Sem sinal" a letra grande vira um traço longo (glifo "—" em 96px); aceitável, pode ser refinado no passo 9.
- Não verificado: leitor de tela real; app Tauri com este componente e sinal real (mic onboard silencioso).
