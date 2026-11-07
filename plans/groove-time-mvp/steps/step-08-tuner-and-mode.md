# Step 8: Afinador + toggle Ativo/Passivo

**Status:** pending
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

- [ ] Com mock: nota e cents atualizam; dentro da tolerância o display entra no estado "afinado".
- [ ] Toggle chama `set_bass_mode` e o `SignalFrame.mode` confirma; falha simulada faz rollback visual.
- [ ] Navegação por teclado e leitores de tela funcionam no toggle.
- [ ] `vue-tsc` limpo.
