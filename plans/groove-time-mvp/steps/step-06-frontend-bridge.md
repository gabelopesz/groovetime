# Step 6: Camada frontend: tipos, composables de evento/invoke

**Status:** pending
**Depends on:** 5

## Goal

Criar a ponte tipada entre Vue e o backend: tipos TS espelhando os payloads Rust, wrappers de `invoke` e composables reativos (`useSignalFrame`, `useEngineStatus`, `useBassMode`) que fazem `listen`/`unlisten` corretamente e alimentam a UI de forma eficiente.

## Scope

**In scope:**
- `src/types/engine.ts`: `SignalFrame`, `PitchInfo`, `BassMode`, `EngineStatus`, `DeviceInfo`, nomes de eventos como constantes (mesmas strings do Rust).
- `src/api/engine.ts`: funções `setBassMode`, `getEngineStatus`, `listInputDevices` (+ `restartEngine` se existir) usando `@tauri-apps/api/core` `invoke` com tipagem e tratamento de erro (`CommandError`).
- `src/composables/useSignalFrame.ts`: `listen` em `onMounted`, `unlisten` em `onUnmounted`; guarda o último frame em `shallowRef` (evitar reatividade profunda a 60 Hz); opcionalmente agrega via `requestAnimationFrame`.
- `src/composables/useEngineStatus.ts` e `useBassMode.ts` (estado otimista + rollback se o invoke falhar).
- Um componente de debug temporário (ou `App.vue` mínimo) exibindo os valores crus para validar a ponte.
- Mock opcional para rodar o frontend no navegador fora do Tauri (`pnpm dev`) com frames sintéticos — útil para os passos 7–9.

**Out of scope:**
- Visual dos componentes (passos 7–9).

## Context

- Tauri v2 JS: `import { invoke } from '@tauri-apps/api/core'` e `import { listen } from '@tauri-apps/api/event'`.
- Os argumentos de `invoke` em Tauri usam camelCase no JS por padrão mapeando para snake_case no Rust.

## Open questions / ambiguities

- Incluir o mock de frames para desenvolvimento no navegador? (Recomendado; confirmar.)
- Gerar os tipos TS automaticamente a partir do Rust (`ts-rs`/`specta`) ou manter manualmente? Plano assume manual (poucos tipos).

## Acceptance criteria

- [ ] `pnpm vue-tsc --noEmit` passa em modo strict.
- [ ] Com o app rodando, o componente de debug mostra pico/nota atualizando suavemente; trocar de rota/desmontar não deixa listeners vazando (verificar contagem de `listen`).
- [ ] Toggle via `useBassMode` reflete no `SignalFrame.mode` vindo do backend.


> Nota do passo 5: chamar `get_engine_status` ao montar (o `engine-status` inicial pode ser emitido antes do `listen`). Eventos: `signal-frame`, `engine-status`; commands: `set_bass_mode`, `get_engine_status`, `list_input_devices`, `restart_engine`. Medir no devtools os ~60 eventos/s.
