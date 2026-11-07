# Step 11: Afinador avançado: A4 ajustável, presets de afinação e modo strobe

**Status:** done
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

- [x] Teste do pipeline Rust (`reference_pitch_changes_note_and_cents`) e teste vitest: 54 Hz → A1 com ~0 cents (A4 = 432) e ~-31.8 cents (A4 = 440).
- [x] `reference_validation` (Rust): 500, 431.9, 446.1, 0, NaN, ∞ rejeitados; 432/440/446 aceitos (`CommandError::InvalidArgument`). `invoke` real só exercitado com valor válido (+1 Hz → 442 no app Tauri, sem erro); o caso 500 está coberto só pelo teste Rust.
- [x] vitest: Drop D com 36.71 Hz → corda D1 (< 1 cent); no padrão 4 cordas, 36.71 Hz fica a ~200 cents de E1 → fora do alcance (null → cai para a nota do backend).
- [x] (parcial) Implementado: fase acumulada em rAF, velocidade ∝ cents (suavizada), zero dentro de ±3 cents, agudo → direita / grave → esquerda; só `transform` escrito direto no elemento. Verificado por screenshots no mock (padrão em posições diferentes; "Agudo +11¢" / "Grave −11¢"). **Não observei o strobe parando em ±3 cents nem medi fps/jank**; `prefers-reduced-motion` cai para a agulha por código (`usePrefersReducedMotion`), sem emular a preferência.
- [x] `cargo clippy --all-targets -D warnings` e `pnpm build` (vue-tsc) limpos; 60 testes Rust + 10 vitest.


> Nota do passo 5: `DspPipeline::set_a4_hz` existe; falta propagar o A4 da UI via atômico em `SharedControls` + command.

## Result

- **Backend:** `SharedControls.reference_bits` (default 440; faixa 432–446 em `A4_MIN_HZ/A4_MAX_HZ`; a constante do padrão vive só em `dsp::note`), loop do engine aplica via `pipeline.set_a4_hz`, `SignalFrame.reference_hz`, `EngineStatus.reference_hz`, command `set_reference_pitch(hz)` (+ `allow-set-reference-pitch` em `build.rs`/capability).
- **Frontend:** `utils/note.ts` (espelha `note.rs`), `utils/tuning.ts` (`nearestString`: corda mais próxima em cents; > 100 cents = fora do alcance → usa a nota cromática do backend), `data/tunings.ts` com presets, `composables/{useTunerSettings,usePrefersReducedMotion}.ts`, `components/{StrobeBand,TunerControls}.vue`; `TunerDisplay` agora recebe `tuning`, `referenceHz`, `view` e mostra a **corda alvo** do preset com cents relativos a ela.
- **Decisões do usuário:** faixa 432–446; presets: 4/5/6 cordas padrão, Drop D, meio tom abaixo (4 e 5); strobe **alterna** com a agulha. Notação com **sustenidos** (D#1 G#1 C#2 F#2; A#0…), coerente com o passo 8. Todos os presets ficam ≥ 29.1 Hz (A#0), acima do `min_freq` de 28 Hz do YIN — teste garante; **Drop A / A0 (27.5 Hz) não é suportado**.
- Cromático (`Cromático (qualquer nota)`) = sem cordas: usa nota/cents do backend, esconde a lista de cordas.
- Introduzi **vitest** (`pnpm test`) para a lógica pura de frontend, pois os critérios (Drop D, A4) pedem verificação e não havia runner.
- Estado de preset/visualização/A4 em `useTunerSettings` é **singleton de módulo** (volta ao default ao reiniciar o app; o passo 12 persiste). O A4 da UI é lido do backend (`get_engine_status`) ao montar.
- Verificado: UI no mock (Chrome real): controles, strobe, stepper de A4 recalculando cents em relação à corda do preset, botão 440; app Tauri real: A4 +1 +1 → 442 sem erro.
- Não verificado: troca de preset pelo `<select>` na UI (só unit tests da lógica), strobe parando em afinado, leitor de tela nos controles, comportamento com baixo real.
