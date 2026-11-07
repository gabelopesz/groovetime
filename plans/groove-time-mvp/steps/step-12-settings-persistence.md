# Step 12: Persistência de preferências (JSON em ~/.config)

**Status:** pending
**Depends on:** 10, 11

## Goal

Lembrar entre sessões o modo Ativo/Passivo, A4, preset de afinação, visualização (agulha/strobe), gate, device preferido e estado do modo mini, num JSON pequeno gravado pelo próprio Rust — sem plugin store e sem novas permissões de filesystem para o frontend.

## Scope

**In scope:**
- `src-tauri/src/settings.rs`: `#[derive(Serialize, Deserialize, Default)] struct Settings { version: u32, bass_mode, reference_hz, tuning_preset: String, tuner_view, gate_dbfs, device_name: Option<String>, mini_mode: bool }` com `#[serde(default)]` para tolerar campos novos/ausentes.
- Caminho: diretório de config do usuário (`app.path().app_config_dir()` do Tauri v2 → `~/.config/<identifier>/settings.json`, ou `~/.config/groove-time/` via crate `dirs` — ver open questions).
- Carregar no `setup` **antes** de iniciar o engine e aplicar (ganho, A4, gate, device preferido para o `find_device` do passo 2).
- Gravação atômica (escrever em `settings.json.tmp` + `rename`), com debounce (~500 ms) para não gravar a cada movimento de slider; feita fora da thread DSP e da callback RT.
- Arquivo corrompido/ilegível → log, renomear para `settings.json.bak`, seguir com defaults (sem crash). Validar faixas ao carregar (mesmas regras dos commands).
- Commands: `get_settings()` e `update_settings(patch)` (parcial) — ou fazer os commands existentes (`set_bass_mode`, `set_reference_pitch`, `set_gate_dbfs`) persistirem automaticamente e expor só `get_settings` + `set_ui_prefs` para o que é só de UI (preset, view). Escolher uma abordagem e manter a superfície de IPC pequena.
- Frontend: ao iniciar, `get_settings()` hidrata os composables (modo, A4, preset, view, gate, mini).

**Out of scope:**
- Sincronização/perfis múltiplos.
- Posição/tamanho da janela (pode usar `tauri-plugin-window-state` depois, mas exige permissões — fora por ora).

## Context

- `serde_json` como nova dependência (pequena). A escrita é feita em Rust, então o frontend não ganha acesso a filesystem — mantém a CSP/capabilities enxutas.
- O device preferido pode não existir no próximo boot (Q-24 desconectada): usar fallback do passo 2 sem sobrescrever a preferência salva.

## Open questions / ambiguities

- Caminho: `app_config_dir()` (usa o identifier, ex. `~/.config/com.groovetime.app/`) ou `~/.config/groove-time/` (mais amigável, exige crate `dirs`)?
- Abordagem de commands: `update_settings(patch)` genérico vs. persistência implícita nos setters existentes?
- O modo Ativo/Passivo deve mesmo ser restaurado? (Risco: abrir com -12 dB num baixo passivo e o usuário não perceber.) Alternativa: restaurar, mas mostrar o modo em destaque ao abrir.

## Acceptance criteria

- [ ] Mudar modo, A4, preset, view, gate → fechar e reabrir o app → tudo restaurado.
- [ ] `settings.json` corrompido manualmente → app abre com defaults e cria `.bak`.
- [ ] Mexer no slider do gate continuamente gera no máximo ~2 gravações/s.
- [ ] Testes unitários: roundtrip serde, `#[serde(default)]` com JSON parcial, validação de faixas ao carregar.
- [ ] Nenhuma permissão nova de fs/plugin nas capabilities; clippy e `vue-tsc` limpos.
