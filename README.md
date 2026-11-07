# Groove Time

Afinador de contrabaixo e monitor de sinal para Linux (Tauri v2 + Rust + Vue 3/TS). Captura a entrada
de um dispositivo (alvo: Input 1 HI-Z da Teyun Q-24), mede pico/RMS em dBFS, detecta a nota com YIN
(até B0 ≈ 30,87 Hz) e mostra um VU meter com alerta PEAK, afinador (agulha ou strobe) e o modo
Ativo/Passivo (-12 dB digitais).

## Desenvolvimento

```sh
pnpm install
pnpm tauri dev      # app completo
pnpm dev            # só o frontend no navegador, com dados simulados (mock)
pnpm test           # testes do frontend (vitest)
cargo test --manifest-path src-tauri/Cargo.toml          # testes do backend
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Testes que exigem um dispositivo de captura ficam marcados `#[ignore]`:
`cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`.

## Modo mini (Ctrl+M)

Encolhe a janela para a nota, os cents e o LED PEAK, **sempre no topo**, e restaura o tamanho anterior
ao sair. A escolha é lembrada entre sessões.

**Wayland:** o protocolo não oferece "sempre no topo" para aplicativos comuns. No GNOME o pedido é
ignorado; no KDE/Hyprland depende de regras do compositor. Em X11 (ou XWayland) funciona. Para rodar
em XWayland, inicie com `GDK_BACKEND=x11`:

```sh
GDK_BACKEND=x11 pnpm tauri dev
```

O app detecta sessões Wayland e mostra um aviso ao lado do botão do modo mini.

## Preferências

Gravadas em `~/.config/com.groovetime.app/settings.json` (modo Ativo/Passivo, A4, afinação,
visualização, gate, device preferido, modo mini). Se o arquivo estiver corrompido, ele é movido para
`settings.json.bak` e o app abre com os padrões.
