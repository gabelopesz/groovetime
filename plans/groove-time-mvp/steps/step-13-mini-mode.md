# Step 13: Modo mini / sempre no topo

**Status:** pending
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

- [ ] Botão/atalho alterna entre normal e mini; ao voltar, tamanho anterior restaurado.
- [ ] Em sessão X11 (ou XWayland), a janela mini fica acima de outra janela em foco.
- [ ] Capabilities não ganharam permissões `core:window:*` além das que já existiam.
- [ ] Modo mini persiste entre reinícios.
- [ ] Clippy e `vue-tsc` limpos.


> Nota do passo 12: `mini_mode` já é persistido (`Settings.mini_mode`, `set_ui_prefs({ mini_mode })`, `get_settings`); falta só a UI/janela. O estado de tamanho/posição da janela não é salvo (fora de escopo do passo 12).
