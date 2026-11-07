# Step 4: Callback RT lock-free + ring buffer + ganho atômico

**Status:** pending
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

- [ ] Teste de integração (pode ser `#[ignore]` por exigir hardware) que inicia captura no device default por 2 s e verifica que amostras chegaram ao consumer e `dropped_samples == 0` com consumidor ativo.
- [ ] Teste unitário da função pura de "extrair canal + aplicar ganho + push" usando um buffer intercalado sintético e um `rtrb` real.
- [ ] Revisão manual: a closure da callback não contém nenhuma das construções proibidas (listar no PR/resumo do passo).
- [ ] Clippy limpo.

## Notes for execution

- Extrair a lógica da callback para uma `fn process_interleaved<T: Sample>(data: &[T], channels, ch_idx, gain, producer: &mut Producer<f32>, dropped: &AtomicU64)` facilita o teste sem hardware.


> Nota do passo 2: PCMs-plugin do ALSA reportam faixas de rate/canais irreais; ao abrir o stream, ler e logar a rate efetiva e tratar falha de build com fallback para outra config.
