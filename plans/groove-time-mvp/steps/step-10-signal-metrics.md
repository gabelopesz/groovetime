# Step 10: Métricas extras de sinal: RMS, contador de clips, noise floor/DC e gate

**Status:** done
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

- [x] Testes unitários passando: RMS de senoide full-scale ≈ -3.01 dBFS (±0.1); contador = 1 por burst (e 2 para 2 bursts) e não conta por amostra/ciclo; histerese; ruído branco -70 dBFS → noise floor ±2 dB; DC 0.02 sob tom de 55 Hz → 2% ±0.3.
- [x] Validação de faixa coberta por teste (`gate_validation`: -200, 0, NaN, ∞ rejeitados; -80/-50/-20 aceitos → `CommandError::InvalidArgument`). `invoke` real exercitado só com valor válido (slider → -70 dBFS sem erro) — **o `invoke` com -200 não foi chamado no app**, apenas o teste Rust. `gated` muda com o gate (teste do pipeline `gate_hides_the_note_and_is_adjustable`).
- [x] No mock o badge zera e desabilita ao clicar (screenshot). `reset_clip_count` no backend: atômico + teste do pipeline (`rms_clip_count_and_dc_in_frame`); não vi o badge zerar no app real (sem clips com o mic em silêncio).
- [x] 62.3 frames/s no engine real (`engine_hw`), 0 drops. `RmsMeter`, `ClipCounter`, `NoiseFloor`, `DcOffset` só usam campos escalares (sem `Vec`/`Box`); revisão manual, sem teste automatizado de alocação.
- [x] `cargo clippy --all-targets -D warnings` e `pnpm build` (vue-tsc) limpos; 57 testes unitários.


> Nota do passo 5: o gate do YIN e o A4 existem no `DspPipeline` (`set_gate_dbfs`, `set_a4_hz`), mas falta um canal atômico em `SharedControls` para o engine aplicá-los em runtime.

## Result

- **DSP novo** (`src-tauri/src/dsp/`): `rms.rs` (EMA de potência, τ = 300 ms, escorvada pelo 1º bloco — sem isso o início "caía" ao piso e contaminava o noise floor), `clip.rs`, `noise.rs`, `dc.rs`. RMS e DC são calculados **na taxa cheia**, por bloco (soma de quadrados/soma), custo desprezível.
- **ClipCounter:** conta evento em pico de bloco ≥ -3 dBFS; só rearma após **100 ms contínuos** abaixo de -6 dBFS (os cruzamentos por zero de uma nota grave, período B0 ≈ 32 ms, contariam vários eventos com a histerese "ingênua" por bloco).
- **NoiseFloor:** mínimo do RMS (sobe no máx. 3 dB/s, desce na hora), só em trechos "quietos" (YIN sem pitch **e** RMS < gate), valor liberado após ≥ 2 s de silêncio acumulado contínuo. **Consequência da janela VU de 300 ms:** depois de parar de tocar, o RMS leva ~3 s para cair abaixo do gate (-50), então o valor aparece ~5 s depois — por isso a dica da UI diz "alguns segundos".
- **Frame/engine:** `SignalFrame` ganhou `rms_dbfs`, `clip_count`, `noise_floor_dbfs`, `dc_offset` (**em %** de full scale), `gated`. `EngineStatus` ganhou `gate_dbfs` (para o slider iniciar no valor certo). `SharedControls` ganhou `gate_bits` (atômico; default -50, faixa -80…-20) e `reset_clips` (AtomicBool consumido com `swap` pela thread DSP). O loop aplica mudanças de gate e reset a cada iteração.
- **Gate:** `gated = rms < gate`; **só esconde a nota** (pitch = None); RMS/ruído/DC continuam (decisão do usuário). O `Yin` recebe o mesmo valor.
- **Commands:** `reset_clip_count`, `set_gate_dbfs(value)` (validação → `CommandError::InvalidArgument`), + `allow-*` em `capabilities/default.json` e `build.rs`.
- **Decisões do usuário:** RMS 300 ms; noise floor = mínimo com decay lento após ~2 s de silêncio; DC em %; gate só esconde a nota; painel de diagnóstico recolhível, fechado por padrão.
- **Frontend:** tipos/API/mock atualizados; `PeakMeter` com faixa RMS (scaleX) e badge "N clips ✕" (botão de reset, desabilitado em 0); `DiagnosticsPanel.vue` (`<details>`): ruído de fundo, DC offset, dicas (ruído > -60 dBFS → cabo/bateria/aterramento; |DC| > 1% → captador/pré-amp; ≤ -110 dBFS → "silêncio digital, confira a entrada"; sem medição → aguardar) e slider do gate (envia ao soltar); `TunerDisplay` mostra "Abaixo do gate" quando `gated`.
- **Verificado:** UI no mock em Chrome real (badge, RMS, reset, slider, diagnóstico); app Tauri real: diagnóstico mostrou ruído -120.0 dBFS (mic onboard suspenso = silêncio digital), DC 0.00 %, gate inicial -50 vindo do backend, slider → -70 dBFS sem erro.
- **Não verificado:** métricas com sinal de baixo real (clip/RMS/DC reais), `invoke` com valor inválido no app, leitor de tela no slider. 
- Ajuda pro passo 11: o A4 ainda não tem atômico (`DspPipeline::set_a4_hz` existe); seguir o mesmo padrão do gate em `SharedControls` (+ `EngineStatus`).
