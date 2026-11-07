# Step 12: Persistência de preferências (JSON em ~/.config)

**Status:** done
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

- [x] No app real: modo Ativo + A4 442 + Strobe → `settings.json` gravado → app fechado e reaberto → tudo restaurado (modo Ativo com aviso em destaque). **Preset e gate não foram alterados pela UI nesse teste** (o `<select>` e o slider não foram acionados); ambos estão cobertos por código/testes (preset via `set_ui_prefs`; gate via `set_gate_dbfs` + teste de debounce), mas não vi a restauração deles no app.
- [x] Corrompido manualmente: app abriu com defaults (440 Hz, Agulha, Passivo), log de erro e `settings.json.bak` criado com o conteúdo original.
- [x] Teste `debounce_limits_writes_during_continuous_updates`: 60 alterações em ~600 ms (debounce de 200 ms no teste) → 1–4 gravações e o último valor chega ao disco. Com o debounce real de 500 ms o limite é ~2/s; não medi arrastando o slider no app.
- [x] 9 testes em `settings.rs`: roundtrip, JSON parcial/campos desconhecidos, sanitização (A4 500, gate -200, preset inválido/`../../etc`, device vazio), enum inválido → erro, arquivo ausente, corrompido → `.bak`, escrita atômica sem `.tmp`, debounce, flush.
- [x] Capabilities só ganharam `allow-get-settings` e `allow-set-ui-prefs` (sem fs/plugin). Clippy `--all-targets -D warnings` e `pnpm build` limpos; 69 testes Rust + 10 vitest.


> Nota do passo 11: o estado de preset/visualização/A4 está em `src/composables/useTunerSettings.ts` (refs de módulo: `presetId`, `view`, `referenceHz`); o gate em `DiagnosticsPanel.vue` (ref local). A persistência precisa restaurar o A4 e o gate no backend via `set_reference_pitch`/`set_gate_dbfs` ao iniciar.

## Result

- **Backend:** `src-tauri/src/settings.rs` (`Settings` com `#[serde(default)]`, `TunerView`, `sanitized()`, `load_or_default`, `write_atomic`, `SettingsStore` com thread `groove-settings` e debounce de 500 ms, `flush()` no encerramento). `AppState` ganhou `settings: OnceLock<Arc<SettingsStore>>` (`init_settings`, `settings()`, `persist()`, `flush_settings()`); `lib.rs` carrega e **aplica** modo/A4/gate **antes** de iniciar o engine e chama `flush_settings` em `RunEvent::Exit`.
- **Decisões do usuário:** caminho = `app_config_dir()` (`~/.config/com.groovetime.app/settings.json`); persistência **implícita** nos setters (`set_bass_mode`, `set_reference_pitch`, `set_gate_dbfs`, `restart_engine`) + `get_settings` + `set_ui_prefs(prefs)` (preset/view/mini, parcial); **restaurar** o modo salvo e mostrá-lo em destaque (aviso azul "Aberto em modo Ativo (−12 dB), como na última sessão." no toggle).
- **Device preferido:** `EngineHandle::start` ganhou `strict: bool`. No boot é `false`: preferência inexistente cai para o device automático (`find_device_or_default`) **sem sobrescrever** o valor salvo. `restart_engine` é `strict` e só grava `device_name` se o engine realmente subiu. **A UI ainda não tem seletor de device**, então `device_name` só muda via `restart_engine` (command existente, sem tela).
- **Frontend:** `getSettings`/`setUiPrefs` (+ mock em memória), tipos `Settings`/`UiPrefs`/`TunerView`; `useTunerSettings.load()` hidrata preset/view/A4 de `get_settings` e **só depois** registra os `watch` que persistem mudanças (evita gravar defaults por cima). Modo e gate já chegam hidratados via `get_engine_status` porque o backend os aplicou no boot.
- **Tolerância/limites:** valores fora de faixa são corrigidos ao carregar; um valor de enum inválido (ex. `"tuner_view": "x"`) invalida o arquivo inteiro (vai para `.bak`) — escolha simples, mas não é tolerante a downgrade. `mini_mode` já existe no JSON/`set_ui_prefs` para o passo 13.
- Teste de integração `engine_hw` ganhou o caso "device preferido inexistente com `strict=false` cai para o automático" (3 testes passam).
- Limpeza: removi `~/.config/com.groovetime.app/` criado pelos meus testes.
- Não verificado: restauração de preset/gate no app, queda do device preferido real (Q-24), erro de escrita (disco cheio/permissão) além do log.
