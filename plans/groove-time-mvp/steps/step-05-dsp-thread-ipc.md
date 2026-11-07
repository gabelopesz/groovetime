# Step 5: Thread DSP, throttling 16 ms, eventos e commands Tauri

**Status:** pending
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

- [ ] `pnpm tauri dev` com device default: no devtools, `listen('signal-frame', ...)` recebe ~60 eventos/s (medir; tolerância 55–65), nunca centenas.
- [ ] Tocar/injetar sinal (ou cantar/assobiar no mic onboard) produz `pitch` coerente; silêncio produz `pitch: null`.
- [ ] `invoke('set_bass_mode', { mode: 'active' })` reduz `peak_dbfs` em ~12 dB para o mesmo sinal.
- [ ] Fechar a janela encerra a thread DSP e o stream sem panic (log de shutdown).
- [ ] Sem device de captura disponível → app abre e emite `engine-status` de erro, sem crash.
- [ ] Clippy limpo; `unwrap`/`expect` ausentes.
