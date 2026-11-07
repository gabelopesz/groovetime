# Step 11: Afinador avançado: A4 ajustável, presets de afinação e modo strobe

**Status:** pending
**Depends on:** 9

## Goal

Tornar a referência A4 ajustável (432–446 Hz), adicionar presets de afinação (4/5/6 cordas, Drop D, meio tom abaixo) com o desvio medido em relação à corda alvo do preset, e oferecer uma visualização "strobe" alternativa à agulha.

## Scope

**In scope:**
- **A4 (Rust)**: `set_reference_pitch(hz: f32)` com validação (432–446, ou a faixa acordada) → `AtomicU32` lido pela thread DSP e passado para `note.rs`. `SignalFrame`/`EngineStatus` incluem `reference_hz`. Adicionar à capability.
- **A4 (UI)**: controle com passos de 1 Hz (stepper ou select) e atalho para voltar a 440.
- **Presets (só frontend)**: `src/data/tunings.ts` (criado no passo 8) com:
  - Padrão 4 cordas: E1 A1 D2 G2
  - Padrão 5 cordas: B0 E1 A1 D2 G2
  - Padrão 6 cordas: B0 E1 A1 D2 G2 C3
  - Drop D (4): D1 A1 D2 G2
  - Meio tom abaixo (4): E♭1 A♭1 D♭2 G♭2 (e 5 cordas: B♭0 …)
  - Seletor de preset; a corda alvo é a mais próxima da `freq_hz` atual, e os cents mostrados são em relação a essa corda (`1200·log2(f / f_alvo)`, com `f_alvo` derivado de A4). Um modo "cromático" (sem preset) usa a nota/cents do backend como antes.
- **Strobe**: `src/components/StrobeBand.vue` — faixa com padrão repetido que desliza com velocidade proporcional aos cents (para quando afinado; o sentido indica sharp/flat). Implementar acumulando a fase num loop `requestAnimationFrame` e aplicando `transform: translateX` (mudar `animation-duration` a cada frame causa saltos). Alternar entre agulha e strobe na UI. Com `prefers-reduced-motion`, cai para a agulha.

**Out of scope:**
- Presets customizáveis pelo usuário (editor de afinação).
- Persistir A4/preset/visualização (passo 12).

## Context

- `note.rs` (passo 3) já recebe A4 como parâmetro; a UI precisa de `reference_hz` para calcular `f_alvo` do preset de forma consistente com o backend.
- No YIN, o `min_freq` (~28 Hz) cobre B♭0 (~29.1 Hz); se Drop/meio tom em 5 cordas descer abaixo disso (ex. A0 = 27.5 Hz), revisar `min_freq` e o tamanho da janela do passo 3.

## Open questions / ambiguities

- Faixa do A4: 432–446 ou mais ampla (415–466 cobre afinação barroca)?
- Lista final de presets (incluir Drop C, B standard em 4 cordas, meio tom abaixo em 5/6 cordas?). Lembrar do limite de `min_freq` acima.
- Strobe como visualização alternativa (alternar) ou ao mesmo tempo que a agulha?
- Notação dos bemóis (E♭ vs D#) — alinhar com a decisão do passo 8.

## Acceptance criteria

- [ ] Senoide de 54 Hz: com A4 = 432 aparece como A1 com ~0 cents; com A4 = 440 aparece como A1 com ~-32 cents.
- [ ] `set_reference_pitch(500)` retorna erro.
- [ ] Preset Drop D: tocar D1 (36.71 Hz) seleciona a corda D1, não E1.
- [ ] Strobe para quando |cents| < tolerância e inverte o sentido entre sharp e flat; sem jank a 60 fps.
- [ ] Clippy e `vue-tsc` limpos.
