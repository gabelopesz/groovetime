# Step 1: Scaffold Tauri v2 + Vue 3/TS + build profile

**Status:** done
**Depends on:** none

## Goal

Criar o esqueleto do projeto Groove Time (Tauri v2, Vue 3 Composition API + TypeScript via Vite, pnpm) que compila e abre uma janela vazia, já com o profile de release otimizado, lints estritos e a base de segurança (capabilities mínimas + CSP).

## Scope

**In scope:**
- Gerar o projeto na raiz do repo (`/home/gabelopesz/Projetos/groovetime`) com `pnpm create tauri-app` (template `vue-ts`) ou equivalente manual; nome do app "Groove Time", identifier tipo `com.groovetime.app` (confirmar).
- `git init` + `.gitignore` (target/, node_modules/, dist/).
- `src-tauri/Cargo.toml`:
  - `[profile.release]` com `lto = true`, `codegen-units = 1`, `strip = true`, `opt-level = 3` (e decidir sobre `panic = "abort"`).
  - Dependências já declaradas para os próximos passos: `cpal` (versão atual estável), `rtrb`, `thiserror`, `anyhow`, `serde` (derive), `log` + `env_logger` ou `tracing` (escolher um). Não usar ainda.
  - `[lints.clippy]` com `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "warn"`. Remover qualquer `.expect()` gerado pelo template (o template do Tauri usa `.expect("error while running tauri application")` em `run()` — substituir por propagação/`log` + `std::process::exit`).
- `tauri.conf.json`: título, tamanho de janela inicial, `app.security.csp` restrita (ex.: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`), sem `devCsp` permissivo além do necessário para o Vite HMR.
- `src-tauri/capabilities/default.json` mínimo: apenas `core:default` reduzido ao necessário (preferir `core:event:default` + `core:window:default` se necessário) — remover `opener`/plugins que o template adiciona e o app não usa.
- Frontend: limpar o boilerplate (remover Greet), `App.vue` vazio com placeholder, `tsconfig` em `strict`.
- Scripts `pnpm tauri dev` / `pnpm tauri build` funcionando.

**Out of scope:**
- Qualquer código de áudio/DSP (passos 2–5).
- Componentes visuais e o mesh gradient (passos 7–9).
- Empacotamento/targets de bundle (passo 10) — pode deixar `bundle.targets` default por ora.

## Context

- Repo vazio exceto `.claude/`. Ferramentas disponíveis: `cargo-tauri 2.7.1`, `pnpm`, `node 26`, `rustc 1.97`. **npm não está instalado** — usar pnpm em tudo.
- `webkit2gtk-4.1` presente (requisito do Tauri v2 no Linux).
- Estrutura alvo sugerida para o Rust (criar módulos vazios só se ajudar; pode deixar para os passos seguintes):
  ```
  src-tauri/src/
    main.rs        // chama groove_time_lib::run()
    lib.rs         // builder Tauri
    audio/         // passo 2 e 4
    dsp/           // passo 3
    engine.rs      // passo 5 (thread DSP + emissão)
  ```

## Open questions / ambiguities

- Identifier do bundle (`com.groovetime.app`? outro domínio do usuário?).
- `panic = "abort"` no release: reduz binário, mas um panic na thread de DSP derruba o app inteiro em vez de só a thread. Confirmar preferência.
- Logging: `log` + `env_logger` (mais simples) ou `tracing` + `tracing-subscriber`? Ou `tauri-plugin-log` (adiciona permissões)?
- Versão do `cpal`: verificar a última no crates.io no momento da execução (0.15.x vs 0.16+/0.17 mudaram APIs de `SampleFormat`/`StreamConfig` e nomes de device).

## Acceptance criteria

- [x] `pnpm install && pnpm tauri dev` abre uma janela "Groove Time" sem erros no console.
- [x] `pnpm tauri build` conclui em modo release.
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passa, com `unwrap_used`/`expect_used` em deny.
- [x] `pnpm vue-tsc --noEmit` (ou `pnpm build`) passa.
- [x] `capabilities/default.json` não contém permissões de plugins não usados; CSP definida em `tauri.conf.json`.

## Notes for execution

- O `create-tauri-app` pode pedir interativamente; usar flags não-interativas (`pnpm create tauri-app groove-time --template vue-ts --manager pnpm --yes` ou similar) e mover para a raiz, ou gerar em subdir temporário e mover os arquivos.
- Não commitar sem o usuário pedir.

## Result

- Scaffold gerado via `create-tauri-app` (vue-ts, pnpm) em dir temporário e movido para a raiz; `git init` + `.gitignore` feitos. Boilerplate (Greet, assets, `tauri-plugin-opener`, `public/`) removido.
- Decisões: identifier `com.groovetime.app`; **sem** `panic = "abort"` (unwind, para um panic na thread de DSP não derrubar o app); logging `log` + `env_logger`.
- Dependências declaradas: `cpal 0.18`, `rtrb 0.4`, `thiserror 2`, `anyhow 1`, `log`, `env_logger`. **cpal está em 0.18** (não 0.15/0.16) — conferir API no passo 2.
- `lib.rs` sem `.expect()`: erro do `run` é logado e `process::exit(1)`.
- Capabilities: só `core:event:default`. CSP definida (e `devCsp` com ws/http do Vite em localhost:1420).
- Verificado: `pnpm build` (vue-tsc + vite), `cargo clippy -D warnings`, `pnpm tauri build` (deb/rpm/AppImage) e `pnpm tauri dev` (compila e roda sem erros; não inspecionei a janela visualmente).
- Janela inicial 480x640 (min 320x400) — ajustar no passo 9/13 se necessário.
