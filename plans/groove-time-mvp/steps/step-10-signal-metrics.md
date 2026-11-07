# Step 10: Métricas extras de sinal: RMS, contador de clips, noise floor/DC e gate

**Status:** pending
**Depends on:** 9

## Goal

Ampliar o monitor de sinal com nível RMS, contador de clips (com reset), diagnóstico de ruído de fundo e DC offset, e um gate configurável que esconde a nota quando o sinal está abaixo de um limiar — tudo calculado na thread DSP e exibido na UI já existente.

## Scope

**In scope:**
- **DSP (`src-tauri/src/dsp/`)**, sem alocação, com testes:
  - `RmsMeter`: RMS em janela deslizante (ex. ~300 ms, comportamento "VU") → dBFS, piso -120.
  - `ClipCounter`: conta **eventos** de cruzamento de -3 dBFS (borda de subida com histerese, ex. só re-arma abaixo de -6 dBFS), não amostras.
  - `NoiseFloor`: estima o ruído de fundo usando o RMS apenas nos trechos em que o YIN retorna `None` e o nível está abaixo do gate (ex. mínimo com decay lento ou percentil baixo). Retorna `Option<f32>` (None até ter amostras suficientes de silêncio).
  - `DcOffset`: média móvel (EMA) do sinal na taxa cheia, reportada em % de full scale (ou dBFS).
- **Engine (passo 5)**: novos campos no `SignalFrame`: `rms_dbfs`, `clip_count: u32`, `noise_floor_dbfs: Option<f32>`, `dc_offset: f32`, `gated: bool`. O gate (`gate_dbfs`, default ~-50 dBFS) vive como `AtomicU32` (bits f32) e é lido pela thread DSP; o reset de clips é um `AtomicBool` consumido pela thread DSP.
- **Commands**: `reset_clip_count()`, `set_gate_dbfs(value: f32)` com validação de faixa (ex. -80…-20; fora disso → `CommandError`). Adicionar à capability.
- **Frontend**:
  - Tipos/API/composables do passo 6 atualizados.
  - `PeakMeter.vue`: barra ou marcador RMS sobreposto ao pico.
  - Badge do contador de clips perto do LED PEAK, com botão de reset.
  - Painel compacto "Diagnóstico": noise floor e DC offset, com dicas simples (ex.: ruído > -60 dBFS → "verifique cabo/bateria/aterramento"; DC > ~1% → "possível problema no captador/pré-amp").
  - Controle do gate (slider) — a nota some quando `gated` é true (o afinador do passo 8 trata como "sem sinal").

**Out of scope:**
- Persistir o gate entre sessões (passo 12).
- A4/presets/strobe (passo 11).

## Context

- RMS e pico vêm do mesmo bloco de amostras na thread DSP; o RMS é calculado na taxa cheia (ou na decimada, se o custo justificar — documentar).
- O gate do YIN (passo 3) já existe internamente; este passo o liga ao atômico e o expõe à UI.
- O noise floor só é útil com as cordas abafadas; a UI pode mostrar "toque nada por 2 s para medir" quando `None`.
- Bateria fraca em baixo ativo costuma aparecer como ruído maior e distorção precoce — é por isso que o diagnóstico ajuda no modo Ativo.

## Open questions / ambiguities

- Janela do RMS: ~300 ms (VU clássico) ou mais curta?
- Algoritmo do noise floor: mínimo com decay ou percentil? Quanto tempo de silêncio até mostrar o valor?
- Unidade do DC offset: % de full scale ou dBFS?
- Default do gate (-50 dBFS?) e se o gate também deve congelar o noise floor/RMS ou só esconder a nota.
- Painel de diagnóstico sempre visível ou recolhível?

## Acceptance criteria

- [ ] Testes: RMS de senoide de amplitude 1.0 ≈ -3.01 dBFS; o contador sobe 1 por burst acima de -3 dB (não por amostra); silêncio + ruído branco a -70 dBFS → noise floor ≈ -70 ±2 dB; sinal com DC de 0.02 → ~2%.
- [ ] `invoke('set_gate_dbfs', { value: -200 })` retorna erro; valor válido muda o comportamento de `gated`.
- [ ] `reset_clip_count` zera o badge.
- [ ] Eventos continuam ~60/s; nenhuma alocação nova nos processadores DSP.
- [ ] Clippy e `vue-tsc` limpos.


> Nota do passo 5: o gate do YIN e o A4 existem no `DspPipeline` (`set_gate_dbfs`, `set_a4_hz`), mas falta um canal atômico em `SharedControls` para o engine aplicá-los em runtime.
