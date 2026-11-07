# Step 6: Camada frontend: tipos, composables de evento/invoke

**Status:** done
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

- [x] `pnpm build` (vue-tsc strict + vite) passa.
- [x] App real (`pnpm tauri dev`, screenshot da janela): origem Tauri, **62 atualizações/s**, status `running`, 2 listeners ativos (contador `activeListenerCount`), estável após `restart_engine`. Pico/nota ficaram em silêncio (mic onboard sem sinal): a variação visual de pico/nota só foi exercitada pelo mock, não verificada em execução. Desmontagem: o código cancela listener/rAF (inclusive se desmontar antes do `listen` resolver), mas sem rota/desmonte real ainda não há teste de runtime.
- [x] Clique em "alternar" → `frame.mode` passou a `active` vindo do backend e `status.mode` também.


> Nota do passo 5: chamar `get_engine_status` ao montar (o `engine-status` inicial pode ser emitido antes do `listen`). Eventos: `signal-frame`, `engine-status`; commands: `set_bass_mode`, `get_engine_status`, `list_input_devices`, `restart_engine`. Medir no devtools os ~60 eventos/s.

## Result

- Criados: `src/types/engine.ts` (tipos manuais + nomes de evento), `src/api/engine.ts` (`setBassMode`, `getEngineStatus`, `listInputDevices`, `restartEngine`, `CommandError`), `src/api/events.ts` (`onSignalFrame`, `onEngineStatus`, `isTauri`, `activeListenerCount`), `src/composables/{useSignalFrame,useEngineStatus,useBassMode}.ts`, `src/mock/engine.ts`, `src/components/DebugPanel.vue` (temporário; `App.vue` só o renderiza).
- Decisões (defaults do plano, sem perguntar): tipos escritos à mão; mock de frames incluído. Fora do Tauri (`isTauri()` falso) a API usa o mock (frames sintéticos a ~60 Hz, plucks com picos acima de -3 dBFS, notas B0–G2, modo Ativo = -12 dB).
- `useSignalFrame`: `shallowRef`, publica por `requestAnimationFrame` e **funde frames** (`mergeFrames`: maior pico + OR de `over_threshold`) para não perder transiente se chegarem 2 eventos entre rAFs.
- `useEngineStatus`: `listen` antes da leitura inicial de `get_engine_status`; evento mais novo vence a leitura. `useBassMode`: estado otimista + rollback; inicia do `mode` do backend.
- Validação real feita com `xdotool`/`import` (screenshot da janela). Não verifiquei o mock no navegador.
- Para o passo 7: `frame.value` pode ser `null` até o primeiro frame; `peak_dbfs` vale -120 em silêncio.
