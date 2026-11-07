# Step 13: Modo mini / sempre no topo

**Status:** done
**Depends on:** 12

## Goal

Adicionar um modo compacto que encolhe a janela para mostrar só a nota, os cents e o LED PEAK, sempre no topo — para deixar num canto enquanto se grava no DAW — e volta ao tamanho anterior ao sair.

## Scope

**In scope:**
- Command Rust `set_mini_mode(enabled: bool)`: no Rust, via `WebviewWindow`, aplica `set_always_on_top`, guarda o tamanho/posição atuais, ajusta `set_min_size` e `set_size` para o compacto (ex. ~260×140), e restaura ao sair. Fazer isso no Rust evita conceder permissões `core:window:*` ao JS.
- Persistir `mini_mode` (passo 12) e restaurar no boot.
- Frontend: layout `MiniView.vue` (nota grande, cents/strobe pequeno, LED PEAK + contador de clips), botão para entrar/sair e atalho de teclado (ex. `M`). Reaproveitar os componentes existentes com variante compacta em vez de duplicar lógica.
- Manter as decorações da janela no modo mini (evita precisar de `data-tauri-drag-region` e da permissão `allow-start-dragging`).

**Out of scope:**
- Janela separada/overlay transparente.
- Lembrar a posição do modo mini em outro canto da tela (além do que o compositor fizer).

## Context

- **Wayland**: o protocolo xdg-toplevel não tem "always on top" para clientes; no GNOME o pedido é ignorado, no KDE/Hyprland depende de regras do compositor. Em X11 (e XWayland com `GDK_BACKEND=x11`) funciona. Documentar no README e, se possível, detectar Wayland e avisar na UI.
- Ao sair do modo mini, restaurar o `min_size` normal definido no passo 9.

## Open questions / ambiguities

- Tamanho do modo mini e o que exatamente mostrar (incluir o toggle Ativo/Passivo?).
- Atalho de teclado (`M`? `Ctrl+M`?).
- Em Wayland: aceitar a limitação com aviso, ou oferecer forçar X11 (`GDK_BACKEND=x11`) via variável no `.desktop`?

## Acceptance criteria

- [x] App real (X11): `Ctrl+M` (XTEST, janela focada) alterna 480×720 → 280×160 → 480×720, mesma posição. O botão "Modo mini" e o botão ⤢ do `MiniView` não foram clicados (só o atalho foi testado no app real); também voltou a 480×720 depois de **reabrir já em mini**.
- [x] Sessão X11 real: `_NET_WM_STATE_ABOVE` presente no mini e removido ao sair; com uma janela vermelha do Chrome em foco sobreposta, o mini aparece por cima (screenshot da tela). **Wayland não testado** (só detecção por testes unitários).
- [x] Capabilities só ganharam `allow-get-window-state` e `allow-set-mini-mode`; nenhum `core:window:*` (a janela é controlada no Rust).
- [x] `mini_mode: true` gravado em `settings.json`; ao reiniciar o app abriu em 280×160 com ACIMA.
- [x] Clippy `--all-targets -D warnings` e `pnpm build` limpos; 71 testes Rust + 10 vitest.


> Nota do passo 12: `mini_mode` já é persistido (`Settings.mini_mode`, `set_ui_prefs({ mini_mode })`, `get_settings`); falta só a UI/janela. O estado de tamanho/posição da janela não é salvo (fora de escopo do passo 12).

## Result

- **Backend:** `src-tauri/src/window.rs` (`apply_mini`, `SavedGeometry`, `WindowState { mini_mode, wayland }`, `is_wayland_session` com override `GDK_BACKEND=x11`), `AppState` guarda `mini_mode` + geometria salva; commands `set_mini_mode(enabled)` (async; aplica na janela, persiste `mini_mode`) e `get_window_state`. No `setup`, se `mini_mode` estava salvo, o mini é aplicado antes de iniciar o engine. `mini_mode` saiu de `set_ui_prefs` (só `set_mini_mode` mexe nele).
- **Decisões do usuário:** mini mostra nota + cents (agulha/strobe compacto) + LED PEAK/clips + botão de sair, ~**280×160**, sem o toggle Ativo/Passivo; atalho **Ctrl+M**; Wayland: **detectar, avisar na UI e documentar**.
- **Bug encontrado e corrigido (GTK/X11):** ao sair do mini, mudar o `min_size` (280×160 → 360×560) fazia o GTK/WM redimensionar para o mínimo e sobrescrever o `set_size` — a janela voltava a **360×560** em vez de 480×720 (inverter a ordem não resolveu). Solução: aplicar min_size + size + posição e **reaplicar o `set_size` ~150 ms depois** (thread curta). É um workaround; foi validado só em X11/este WM.
- **Frontend:** `useMiniMode` (estado do módulo, backend como fonte da verdade), `MiniView.vue`, variantes `compact` em `TunerDisplay` e `PeakMeter` (reaproveitam a lógica), botão "Modo mini Ctrl+M" na tela normal + aviso de Wayland, `Ctrl+M` global (`keydown`). Mock: `?wayland` na URL simula o aviso.
- `README.md` criado na raiz (dev/testes, modo mini e limitação do Wayland com `GDK_BACKEND=x11`, onde ficam as preferências).
- Não verificado: Wayland real; clique nos botões do mini; o aviso de Wayland na UI (só mock/código); posição restaurada fora do mesmo monitor; leitor de tela.
- Limpeza: removi `~/.config/com.groovetime.app/` criado nos testes.
