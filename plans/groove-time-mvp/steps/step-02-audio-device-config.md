# Step 2: Seleção de device e config de captura (cpal/ALSA)

**Status:** done
**Depends on:** 1

## Goal

Implementar o módulo `audio::device` que, usando a host API ALSA do `cpal`, enumera devices de captura, encontra a Teyun Q-24 (com fallback para o default), escolhe a melhor configuração suportada (alvo 192 kHz, formato de 24 bits/I32 ou F32) e retorna tudo com erros tipados via `thiserror`.

## Scope

**In scope:**
- `src-tauri/src/audio/mod.rs` e `audio/device.rs` (e `audio/error.rs` se fizer sentido).
- `enum AudioError` com `thiserror` (ex.: `HostUnavailable`, `NoInputDevice`, `DeviceNotFound(String)`, `UnsupportedConfig`, `BuildStream(#[from] cpal::BuildStreamError)`, `PlayStream(...)`, `DefaultConfig(...)`, `DevicesError(...)`).
- Função `select_host()` → `cpal::host_from_id(cpal::HostId::Alsa)`.
- Função `list_input_devices() -> Result<Vec<DeviceInfo>, AudioError>` (nome + canais + sample rates suportados) — será exposta como command no passo 5.
- Função `find_device(preference: Option<&str>) -> Result<cpal::Device, AudioError>`: match por substring case-insensitive ("Q-24", "Q24", "Teyun" — confirmar o nome real que o ALSA/PipeWire expõe), fallback para `default_input_device()`.
- Função `choose_config(device) -> Result<CaptureConfig, AudioError>`: percorre `supported_input_configs()`, prioriza: (1) rate 192000, (2) formato `I32` (24-bit empacotado em 32) ou `F32`, depois `I16`; canais ≥ 1. Retorna `StreamConfig` + `SampleFormat` + `channels` + `sample_rate` + índice do canal alvo (default 0 = Input 1 HI-Z).
- Testes unitários da lógica de escolha de config sobre listas sintéticas (separar a função de ranking da chamada ao cpal para ser testável sem hardware).

**Out of scope:**
- Construir/rodar o stream e a callback (passo 4).
- Expor commands Tauri (passo 5).
- UI de seleção de device (pode ser futura; o command de listagem fica no passo 5).

## Context

- Ambiente de planejamento: PipeWire ativo com sources a **48 kHz s32le**. Pelo ALSA do cpal, os devices visíveis normalmente são `default`/`pipewire` (via plugin `pipewire-alsa`, que resampleia) e os `hw:CARD=...`/`plughw:...` diretos.
- A Q-24 não estava conectada durante o planejamento; o executor deve, se possível, pedir ao usuário para conectá-la e rodar `arecord -l`, `arecord -L | grep -i -A2 q` e `pactl list short sources` para descobrir o nome real e as rates/formatos suportados (`arecord -D hw:X --dump-hw-params`).
- 24-bit: no ALSA a Q-24 provavelmente expõe `S24_3LE` ou `S32_LE`. O cpal não tem `I24` em todas as versões — verificar a versão escolhida no passo 1. Se só houver `I32`, os 24 bits ficam nos bits altos (escala normal de I32 → f32 funciona).
- API de nomes de device mudou em versões recentes do cpal (`name()` deprecado em favor de `description()`/`id()` em algumas versões) — checar docs da versão usada.

## Open questions / ambiguities

- **Caminho para 192 kHz (decisão importante):**
  a) Abrir `hw:`/`plughw:` da Q-24 direto → 192 kHz real, mas tira o device do PipeWire enquanto o app roda (outros apps não usam a placa ao mesmo tempo).
  b) Usar o PCM `pipewire`/`default` e instruir o usuário a configurar `default.clock.allowed-rates = [ 48000 96000 192000 ]` (e/ou `default.clock.rate`) em `~/.config/pipewire/pipewire.conf.d/` — compartilhado, mas depende de config externa.
  c) Ambos: preferir (b), oferecer (a) como "modo exclusivo".
  Confirmar com o usuário. Note que para afinação 48 kHz é mais que suficiente; 192 kHz só importa como requisito de fidelidade/compatibilidade.
- Nome exato com que a Q-24 aparece no ALSA (depende do hardware conectado).
- Se 192 kHz não estiver disponível: cair para a maior rate suportada silenciosamente (com log) ou reportar erro? Sugestão: fallback com log + informar na UI a rate efetiva.
- A Q-24 é stereo-in (Input 1 HI-Z, Input 2)? Confirmar número de canais e qual índice é o HI-Z.

## Acceptance criteria

- [x] `cargo test` cobre a função de ranking de configs (192k preferido; fallback para maior rate; formato I32/F32 preferido sobre I16).
- [x] Um teste/binário de debug (ex.: `cargo run --example list_devices` ou teste `#[ignore]`) lista devices de captura e a config escolhida na máquina atual sem panic.
- [x] Nenhum `unwrap`/`expect` fora de testes; clippy limpo.

## Notes for execution

- Mantenha o tipo `CaptureConfig` simples e `Clone`, ele será passado para o passo 4 e serializado (parcialmente) para a UI no passo 5 — derive `serde::Serialize` num `DeviceInfo`/`StreamInfo` separado.

## Result

- Criados `src-tauri/src/audio/{mod,error,device}.rs` e `src-tauri/examples/list_devices.rs` (`cargo run --example list_devices [preferencia]`).
- Decisões do usuário: **caminho 192k = ambos, preferindo PipeWire** (sem preferência usa Q-24/Teyun → default; `find_device(Some("hw:"))` dá o modo exclusivo por substring de nome/id); **fallback** = maior rate disponível + `log::warn`, com `StreamInfo.below_target_rate` para a UI.
- API cpal 0.18: `description().name()`/`id()` (não há mais `name()`), `SampleFormat::I24` existe (4 bytes, 24 bits úteis), erro único `cpal::Error`, `HostId::Alsa`.
- Ranking puro em `rank_configs` (rate 192k > maior rate; depois I32/I24 > F32 > I16; canais > `target_channel`); 6 testes passam. Clippy `--all-targets -D warnings` limpo.
- **Achado para passos 4/14:** os PCMs-plugin do ALSA (`default`, `pipewire`, `plughw`) reportam faixas irreais (1..u32::MAX Hz, 32–64 canais), então "192 kHz escolhido" aqui não garante a rate real do stream. Na máquina de teste a escolha foi 192000/1ch/I24 no `default`. A rate efetiva deve ser conferida ao construir o stream (passo 4) e validada com a Q-24 real (passo 14). Nome real da Q-24 e número/índice dos canais continuam sem confirmação (placa não conectada).
