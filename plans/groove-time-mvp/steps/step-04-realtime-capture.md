# Step 4: Callback RT lock-free + ring buffer + ganho atômico

**Status:** done
**Depends on:** 2, 3

## Goal

Construir e iniciar o stream de captura `cpal` com uma callback de tempo real que extrai o canal alvo (Input 1 / HI-Z), converte para f32, aplica o ganho Ativo/Passivo lido atomicamente e envia as amostras para um ring buffer `rtrb` SPSC — sem locks, sem alocação, sem panics.

## Scope

**In scope:**
- `src-tauri/src/audio/capture.rs`:
  - `struct SharedControls { gain_bits: AtomicU32, channel_index: AtomicUsize?, dropped_samples: AtomicU64, ... }` em `Arc`, criada fora da callback. Ganho como `f32::to_bits` (1.0 passivo, `10^(-12/20) ≈ 0.2512` ativo).
  - `fn start_capture(device, CaptureConfig, controls: Arc<SharedControls>) -> Result<(cpal::Stream, rtrb::Consumer<f32>), AudioError>`.
  - Callback genérica sobre o tipo de amostra (`I32`, `F32`, `I16` via `cpal::Sample`/`FromSample<f32>`), iterando `data.chunks_exact(channels)`, pegando `frame[channel_index]`, multiplicando pelo ganho, `producer.push(x)`; em buffer cheio, incrementar contador de drops (não bloquear, não logar dentro da callback).
  - Capacidade do ring: ex. ~0.5 s de áudio na rate real (192k → 96k amostras), alocado antes do stream.
  - Callback de erro do stream: sinaliza via atômico/flag (ex. `stream_error: AtomicBool`) — também sem alocação; a thread DSP reporta à UI.
  - Ownership do `cpal::Stream` (não é `Send` em todas as plataformas/versões): definir onde ele vive (thread dedicada que o mantém vivo e encerra via canal, ou guardado em estado Tauri se for `Send` no ALSA).
- Considerar `rtrb::Producer::write_chunk_uninit` para escrever em lote (mais eficiente que push por amostra), se a API ajudar.

**Out of scope:**
- Consumo do ring, DSP e emissão de eventos (passo 5).
- Commands Tauri (passo 5).

## Context

- Regras rígidas (requisito do usuário): NADA de `Mutex`/`RwLock`, `Vec::new`, `String`, `format!`, `println!`/`log!`, `Box::new`, `unwrap`/`expect` dentro da callback de dados.
- O ganho -12 dB é aplicado aqui ("na thread do Rust", requisito 4) para que pico e afinador reflitam o sinal atenuado — confirma o propósito de compensar pré-amps ativos e não estourar o -3 dBFS.
- `cpal::Stream` no Linux/ALSA: verificar se é `Send` na versão usada; caso não seja, criar uma "audio owner thread" que constrói o stream e espera um sinal de shutdown (`std::sync::mpsc` fora do caminho RT é ok).

## Open questions / ambiguities

- Mudança de device/canal em runtime: suportar (rebuild do stream) ou só na inicialização? Sugestão: o canal via atômico em runtime; device só via restart do engine (passo 5).
- Precisa de um "downmix" opcional (Input 1+2) ou estritamente um canal? Assumido: um canal.
- O ganho deve ter rampa (evitar "click" visual)? Para medição não importa; assumido salto direto.

## Acceptance criteria

- [x] Teste de integração (pode ser `#[ignore]` por exigir hardware) que inicia captura no device default por 2 s e verifica que amostras chegaram ao consumer e `dropped_samples == 0` com consumidor ativo.
- [x] Teste unitário da função pura de "extrair canal + aplicar ganho + push" usando um buffer intercalado sintético e um `rtrb` real.
- [x] Revisão manual: a closure da callback não contém nenhuma das construções proibidas (listar no PR/resumo do passo).
- [x] Clippy limpo.

## Notes for execution

- Extrair a lógica da callback para uma `fn process_interleaved<T: Sample>(data: &[T], channels, ch_idx, gain, producer: &mut Producer<f32>, dropped: &AtomicU64)` facilita o teste sem hardware.


> Nota do passo 2: PCMs-plugin do ALSA reportam faixas de rate/canais irreais; ao abrir o stream, ler e logar a rate efetiva e tratar falha de build com fallback para outra config.

## Result

- Criado `src-tauri/src/audio/capture.rs` (+ reexports em `audio/mod.rs`) e `src-tauri/tests/capture_hw.rs` (`#[ignore]`: `cargo test --test capture_hw -- --ignored --nocapture`).
- API: `SharedControls` (atômicos: `gain_bits`, `channel_index`, `dropped_samples`, `stream_error`; `set_active_mode(bool)` → `GAIN_PASSIVE = 1.0` / `GAIN_ACTIVE = 0.25118864` = -12 dB); `start_capture(&Device, CaptureConfig, Arc<SharedControls>) -> Result<Capture, AudioError>`; `Capture { stream, consumer: Consumer<f32>, controls, config }` (`config` é a configuração efetivamente usada).
- **Decisões (defaults do plano):** canal alvo mutável em runtime via atômico (clamp ao nº de canais); device só muda reiniciando; um canal só (sem downmix); ganho sem rampa. **`cpal::Stream` é `Send + Sync` no ALSA** (asserção do próprio cpal), então não há thread dona: o stream vive em `Capture`; soltar a struct encerra a captura.
- Usei `build_input_stream_raw` + `Data::as_slice::<T>()` (evita o `expect` interno do `build_input_stream`); formatos suportados: I16, I24, I32, F32, F64. Ring de 0.5 s (rate × 0.5 amostras). Push por amostra (não usei `write_chunk_uninit`; drops contados localmente e somados 1× por callback).
- **Fallback (nota do passo 2):** se a config escolhida falhar ao abrir, cai para `default_input_config()` do device e loga; a rate efetiva fica em `Capture.config`.
- **Revisão da callback RT** (closure em `build_stream` + `process_interleaved`): contém apenas `as_slice` (sem alocação), loads/stores/`fetch_add` atômicos Relaxed, `chunks_exact`, `get`, `min`, multiplicação e `Producer::push`. Sem `Mutex`/`RwLock`, `Vec`/`String`/`Box`/`format!`, log/print, `unwrap`/`expect` (e `chunks_exact(0)` protegido por guarda).
- Verificação: 31 testes unitários (5 novos: extração de canal + ganho, conversão I16/I32, drops com ring cheio, clamp de canal/0 canais, ganho/modo); clippy `--all-targets -D warnings` limpo; teste de integração rodou no `default` (PipeWire): ~370k amostras em 2 s a 192000 Hz, 1 canal, 0 drops, sem erro de stream. **O PipeWire entregou 192 kHz de fato** nesta máquina; falta confirmar com a Q-24 real (passo 14).
- Para o passo 5: o consumidor precisa drenar o ring continuamente; ler `has_stream_error()` periodicamente; criar o `Yin` com `Decimator::output_rate()`.
