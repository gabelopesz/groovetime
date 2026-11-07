# Plano: Groove Time — MVP (afinador de contrabaixo + monitor de sinal)

## Goal

Construir o "Groove Time", app desktop Linux (Tauri v2 + Rust + Vue 3/TS) que captura o Input 1 (HI-Z) da interface Teyun Q-24, calcula pico em dBFS e pitch (YIN, até B0 ≈ 30.87 Hz) numa thread de DSP e exibe um VU meter estilo LED "PEAK" (alerta acima de -3 dBFS), um afinador e um toggle Ativo/Passivo (-12 dB digital), sobre um fundo mesh gradient 100% CSS.

## Context & key decisions

- **Repo greenfield**: em 2026-11-06 o diretório só continha `.claude/`. Sem git inicializado.
- **Toolchain encontrada na máquina** (Manjaro, kernel 6.18): `rustc 1.97`, `cargo-tauri 2.7.1` instalado, `node 26`, **`pnpm`** (npm/bun ausentes → usar pnpm), `alsa-lib 1.2.16`, `libpipewire 1.6.8`, `webkit2gtk-4.1 2.52`.
- **A Q-24 NÃO estava conectada durante o planejamento** (`arecord -l` só mostrou a placa onboard HDA Intel). Todo o código deve funcionar com qualquer device de captura (fallback), e a validação com hardware real fica concentrada no passo 10.
- **PipeWire roda a 48 kHz por padrão** (`pactl` mostrou `s32le 2ch 48000Hz`). Pegar 192 kHz exige ou abrir o device `hw:`/`plughw:` da Q-24 direto via ALSA (bypass do PipeWire, exclusivo) ou configurar `default.clock.allowed-rates` no PipeWire. Essa decisão está registrada como open question no passo 2.
- **Arquitetura de threads** (decidida no plano):
  1. callback `cpal` (RT): extrai canal 1, converte p/ f32, aplica ganho lido de `AtomicU32` (bits de f32), faz `push` num ring buffer `rtrb` SPSC. Sem locks, sem alocação, sem `unwrap`.
  2. thread DSP: consome o ring, calcula pico (com hold/decay), decima o sinal (anti-alias + downsample) e roda YIN; publica um snapshot a cada ~16 ms via `app_handle.emit`.
  3. thread principal Tauri: commands (`invoke`) mínimos.
- **YIN a 192 kHz é caro** (período de B0 ≈ 6220 amostras → janela ≥ 12.4k amostras, O(N²)). Decidido: decimar para ~4–8 kHz antes do YIN (o fundamental de baixo de 5/6 cordas fica < ~400 Hz). Pico é calculado na taxa cheia.
- **Erros**: `thiserror` para tipos de erro do módulo de áudio; `anyhow` permitido só na borda (`main`/setup). Proibido `unwrap()`/`expect()` em código de produção (testes podem usar). Habilitar `clippy::unwrap_used` e `clippy::expect_used` como `deny`.
- **Release profile**: `lto = true`, `codegen-units = 1`, `strip = true`, `opt-level = 3`, `panic = "abort"` (avaliar).
- **Segurança**: capabilities Tauri v2 mínimas (só `core:event:default` + commands próprios), CSP restrita sem `unsafe-eval`, sem plugins desnecessários (shell, fs, http etc.).
- **Extras (passos 10–13)**, adicionados a pedido do usuário depois do plano inicial: RMS, contador de clips, noise floor/DC offset, gate configurável, A4 ajustável, presets de afinação, modo strobe, persistência de preferências e modo mini. Ficam depois do layout (passo 9) para não atrasar o núcleo, mas os passos 3 e 8 já preparam os ganchos (A4 e gate como parâmetros; cordas como dados).
- **Idioma**: UI e comentários podem ser em português; identificadores de código em inglês.
- **Fora de escopo do plano todo**: Windows/macOS, gravação de áudio, monitoramento/saída de áudio (playback), afinações alternativas customizáveis além de um preset padrão (pode virar plano futuro), auto-update.

## Steps

| # | Step | Status | Depends on |
|---|------|--------|------------|
| 1 | [Scaffold Tauri v2 + Vue 3/TS + build profile](steps/step-01-scaffold.md) | done | — |
| 2 | [Seleção de device e config de captura (cpal/ALSA)](steps/step-02-audio-device-config.md) | done | 1 |
| 3 | [Biblioteca DSP pura: pico dBFS, decimação e YIN](steps/step-03-dsp-core.md) | done | 1 |
| 4 | [Callback RT lock-free + ring buffer + ganho atômico](steps/step-04-realtime-capture.md) | done | 2, 3 |
| 5 | [Thread DSP, throttling 16 ms, eventos e commands Tauri](steps/step-05-dsp-thread-ipc.md) | done | 4 |
| 6 | [Camada frontend: tipos, composables de evento/invoke](steps/step-06-frontend-bridge.md) | done | 5 |
| 7 | [Componente VU meter / LED PEAK](steps/step-07-vu-meter.md) | pending | 6 |
| 8 | [Afinador + toggle Ativo/Passivo](steps/step-08-tuner-and-mode.md) | pending | 6 |
| 9 | [Mesh gradient CSS + layout final](steps/step-09-mesh-gradient-layout.md) | pending | 7, 8 |
| 10 | [Métricas extras: RMS, contador de clips, noise floor/DC, gate](steps/step-10-signal-metrics.md) | pending | 9 |
| 11 | [Afinador avançado: A4 ajustável, presets, strobe](steps/step-11-tuner-extras.md) | pending | 9 |
| 12 | [Persistência de preferências (JSON em ~/.config)](steps/step-12-settings-persistence.md) | pending | 10, 11 |
| 13 | [Modo mini / sempre no topo](steps/step-13-mini-mode.md) | pending | 12 |
| 14 | [Hardening de segurança, empacotamento Linux e validação na Q-24](steps/step-14-security-packaging-validation.md) | pending | 10, 11, 12, 13 |

> Status values: `pending`, `in-progress`, `done`, `blocked`.

Passos 2 e 3 são independentes entre si (ambos dependem só do 1). Passos 7 e 8 também, assim como 10 e 11.

## How to execute this plan

Run the `execute-plan-step` skill (e.g. "execute the next step of the `groove-time-mvp` plan"). It will:
1. Pick the next `pending` step whose dependencies are `done`.
2. Enter plan mode to surface and resolve that step's open questions with you before writing any code.
3. Implement just that step, then mark it `done` in this table and in the step file.

Call it again to move on to the following step — steps are executed one at a time, not all at once.
