# Step 5: Thread DSP, throttling 16 ms, eventos e commands Tauri

**Status:** done
**Depends on:** 4

## Goal

Criar o "engine": uma thread dedicada que consome o ring buffer, alimenta peak meter, decimador e YIN, agrega os resultados e emite **no máximo um** evento Tauri a cada ~16 ms com um snapshot (pico, hold, alerta, frequência, nota, cents, modo); e expor o conjunto mínimo de commands para o frontend.

## Scope

**In scope:**
- `src-tauri/src/engine.rs`:
  - `Engine::start(app_handle, device_pref) -> Result<EngineHandle, AudioError>` que usa passos 2 e 4, cria a thread DSP e guarda handles para shutdown (flag atômica + `join`).
  - Loop DSP: drena o consumer em blocos (`read_chunk`), processa peak na taxa cheia, decima e acumula numa janela circular para YIN (hop ex. ~16–32 ms), e a cada `Instant` ≥ 16 ms emite o snapshot. Dormir/esperar de forma barata quando o ring estiver vazio (ex. `thread::sleep(2ms)` ou `park_timeout`).
  - Estabilização da leitura de pitch (mediana de 3–5 estimativas ou EMA em cents; esconder nota quando sinal some, com pequeno hold).
  - Payload `#[derive(Serialize, Clone)] struct SignalFrame { peak_dbfs: f32, peak_hold_dbfs: f32, over_threshold: bool, clipped_recently: bool, pitch: Option<PitchInfo { freq_hz, note, octave, cents }>, mode: BassMode, sample_rate: u32, dropped_samples: u64 }` emitido como evento `"signal-frame"` (nome em constante).
  - Evento separado `"engine-status"` (start, erro de stream, device desconectado) — emitido raramente.
- Commands em `src-tauri/src/commands.rs` (registrados com `tauri::generate_handler!`):
  - `set_bass_mode(mode: "active" | "passive")` → escreve o ganho atômico.
  - `get_engine_status()` → device, rate, formato, canal, modo atual.
  - `list_input_devices()` (do passo 2) e, se aprovado, `restart_engine(device: Option<String>)`.
  - Retornos `Result<T, CommandError>` onde `CommandError` é `Serialize` (mapeando `AudioError` para mensagem).
- Estado gerenciado com `app.manage(...)` contendo o `Arc<SharedControls>` e o handle do engine. Locks são permitidos aqui (thread principal, não RT), mas preferir atômicos quando simples.
- Iniciar o engine no `setup` do Tauri; falha de áudio NÃO derruba o app — vira `engine-status` de erro para a UI.
- Capabilities: adicionar permissões geradas para os commands próprios (Tauri v2 com `build.rs` + `permissions/` ou `allow-<command>` via `tauri::generate_handler` + capability) — somente os commands acima.

**Out of scope:**
- Qualquer código Vue (passo 6+).
- Hardening final de CSP/capabilities e empacotamento (passo 10).

## Context

- Requisito: throttling — a UI só precisa de 60 fps. O pico emitido deve ser o **máximo** do intervalo de 16 ms (não a última amostra) para nunca perder um transiente de clip.
- Em Tauri v2: `use tauri::Emitter; app_handle.emit("signal-frame", &frame)`. A alocação/serialização acontece aqui, na thread DSP (não RT) — permitido.
- Commands próprios em Tauri v2 precisam de permissão explícita na capability se o app definir `permissions` para eles; documentar a abordagem escolhida (ex.: `src-tauri/permissions/default.toml` com `commands.allow = [...]` e `build.rs` usando `tauri_build::Attributes::new().app_manifest(AppManifest::new().commands(&[...]))`).

## Open questions / ambiguities

- Modo default ao abrir: passivo (0 dB)? (Persistência entre sessões é tratada no passo 12 — aqui só o default.)
- Expor `restart_engine`/seleção de device agora ou deixar para depois? Afeta superfície de IPC.
- Hold do "clipped_recently" (ex.: LED fica aceso 1–2 s após cruzar -3 dB, como LEDs de hardware) — definir valor junto com o passo 7.
- Comportamento ao desconectar a Q-24 com app aberto: tentar reconectar automaticamente (polling) ou só mostrar erro + botão de reiniciar?

## Acceptance criteria

- [x] ~60 eventos/s: medido no engine real (teste `engine_hw`): **62.3 frames/s**. *Não* medido no devtools do webview (sem UI ainda) — conferir ao fazer o passo 6.
- [x] Pitch coerente: validado com sinais sintéticos a 192 kHz no `DspPipeline` (A1 < 2 cents; silêncio → `None`). Não testado com mic/baixo real.
- [x] Modo ativo reduz ~12 dB: ganho -12 dB testado no pipeline (diferença 12.0 ± 0.1) e `mode` refletido nos frames do engine real. O `invoke` em si só será exercitado no passo 6.
- [x] Shutdown: `stop()` junta a thread e solta o stream em ~3.6 ms sem panic; `RunEvent::Exit` chama `AppState::stop()`.
- [x] Sem device → `EngineHandle::start` retorna `Err` (teste) e `AppState::start` converte em `engine-status` de erro; app abriu em `pnpm tauri dev` sem erros no log.
- [x] Clippy `--all-targets -D warnings` limpo; sem `unwrap`/`expect` fora de testes.

## Result

- Novos: `src/pipeline.rs` (`DspPipeline`, `SignalFrame`, `PitchInfo`, `BassMode`), `src/dsp/smooth.rs` (`PitchSmoother`: mediana de 5 + hold de 10 leituras ≈ 160 ms), `src/engine.rs` (`EngineHandle`, `AppState`, eventos), `src/commands.rs`, `build.rs` com `AppManifest`, `tests/engine_hw.rs` (`#[ignore]`). `lib.rs` agora monta o app, gerencia `Arc<AppState>` e para o engine em `RunEvent::Exit`. 40 testes unitários passam.
- **Decisões do usuário:** expor `list_input_devices` + `restart_engine`; desconexão da placa = erro em `engine-status` + reinício manual (sem reconexão automática). **Defaults meus:** modo inicial passivo; `clipped_recently` com hold de 1.5 s (`CLIP_HOLD_SECS`, contado pelo tempo de áudio processado — pode ser ajustado no passo 7).
- Eventos: `signal-frame` (≤ 1 por 16 ms, agendamento fixo sem rajadas; pico = **máximo do intervalo**, `over_threshold` = qualquer pico ≥ -3 dBFS no intervalo) e `engine-status` (`state`: starting/running/error/stopped, `message`, `stream`, `mode`).
- Commands: `set_bass_mode("active"|"passive")`, `get_engine_status`, `list_input_devices`, `restart_engine(device?)`. Erros viram string via `CommandError`. Permissões: `AppManifest` gera `allow-<command>`, listadas em `capabilities/default.json` junto com `core:event:default`.
- O engine é iniciado numa thread no `setup` (enumeração ALSA é lenta). **Como o `engine-status` inicial pode ser emitido antes do webview escutar, o frontend (passo 6) deve chamar `get_engine_status` ao montar.**
- Gate do YIN fixo em -50 dBFS (RMS pós-ganho/decimação); A4 fixo em 440 Hz (`set_a4_hz`/`set_gate_dbfs` já existem no pipeline para os passos 10/11, mas ainda não há atômicos em `SharedControls` para alterá-los em runtime — o passo 10/11 precisa adicioná-los).
- Na máquina de teste o device `default` do PipeWire entregou 192 kHz/I24/mono, 0 drops.
