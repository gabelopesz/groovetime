# Step 14: Hardening de segurança, empacotamento Linux e validação na Q-24

**Status:** pending
**Depends on:** 10, 11, 12, 13

## Goal

Revisar e travar a superfície de segurança (capabilities, CSP, commands), gerar os pacotes Linux para CachyOS/Manjaro/Arch e Ubuntu 24.04, e validar o app end-to-end com a Teyun Q-24 real e um contrabaixo (passivo e ativo).

## Scope

**In scope:**
- **Segurança**:
  - Auditar `src-tauri/capabilities/*.json`: só `core:event` necessário (listen/unlisten), permissões de janela estritamente usadas e os commands próprios (`set_bass_mode`, `get_engine_status`, `list_input_devices`, `reset_clip_count`, `set_gate_dbfs`, `set_reference_pitch`, `get_settings`, `update_settings`, `set_mini_mode`, …). Confirmar que o modo mini não exigiu permissões `core:window:*` no JS (a manipulação da janela é feita no Rust). Nada de shell/fs/http/opener.
  - CSP final em `tauri.conf.json` sem `unsafe-eval`, sem origens externas; avaliar remover `'unsafe-inline'` de `style-src` (Vue com estilos inline/`:style` exige atenção — usar CSS vars via classes ou aceitar `'unsafe-inline'` só para estilos, documentando).
  - Validar inputs dos commands no Rust (enum para modo, limites de nome de device).
  - Desabilitar devtools em release; `withGlobalTauri: false`.
- **Empacotamento**:
  - `bundle.targets`: `deb` (Ubuntu 24.04), `appimage`, e `rpm` se desejado; para Arch/Manjaro/CachyOS escrever um `PKGBUILD` (em `packaging/arch/`) que compila do source ou reempacota o binário/AppImage.
  - Declarar dependências de runtime no deb (`libwebkit2gtk-4.1-0`, `libasound2t64` no 24.04, `libgtk-3-0`) e no PKGBUILD (`webkit2gtk-4.1`, `alsa-lib`, `gtk3`, opcional `pipewire-alsa`).
  - Ícones do app (gerar com `pnpm tauri icon`).
  - Documentar no README do projeto: build, dependências, e a config de PipeWire para 192 kHz (se esse caminho foi escolhido no passo 2).
- **Validação com hardware** (com o usuário):
  - Q-24 conectada: device detectado automaticamente; rate efetiva = 192 kHz (ou a decidida); Input 1 HI-Z usado.
  - Baixo passivo e ativo: afinação de B0/E1/A1/D2/G2 comparada com um afinador de referência (desvio < ~2 cents).
  - LED PEAK do app acende coerente com o LED físico da placa (ajuste de ganho da placa no limiar); modo Ativo reduz ~12 dB.
  - Medir: CPU do processo, `dropped_samples` = 0 em 10 min, eventos ~60/s, tamanho do binário release.
  - Funcionalidades extras: RMS e contador de clips coerentes; noise floor com cordas abafadas (passivo vs ativo, bateria fraca se possível); A4 em 432 e 442 confere com o afinador de referência; presets Drop D/meio tom abaixo; strobe para quando afinado; preferências sobrevivem a reinício; modo mini fica no topo (X11 e Wayland).
  - Testar em pelo menos Manjaro (máquina atual); Ubuntu 24.04 via VM/container se possível.

**Out of scope:**
- Publicação em AUR/PPA/Flathub (pode virar plano futuro).
- Assinatura de pacotes / auto-update.

## Context

- Depende de todos os passos anteriores estarem prontos. A Q-24 não estava conectada no planejamento; agendar com o usuário a sessão de validação.
- AppImage com WebKitGTK tem problemas conhecidos em algumas distros (ex. EGL/Wayland, `WEBKIT_DISABLE_DMABUF_RENDERER=1`); documentar workarounds se aparecerem.

## Open questions / ambiguities

- Quais formatos de pacote são realmente desejados (deb + AppImage + PKGBUILD? rpm? Flatpak — exigiria permissões de áudio via portal/PipeWire)?
- PKGBUILD compilando do source (`groove-time`) ou binário (`groove-time-bin`)?
- Há um afinador de referência disponível para a validação (pedal/clip-on/app)?
- Aceitar `'unsafe-inline'` em `style-src`?

## Acceptance criteria

- [ ] `pnpm tauri build` gera `.deb` e `.AppImage` (e demais formatos acordados); `makepkg` funciona com o PKGBUILD.
- [ ] Capabilities e CSP revisadas e documentadas; nenhum plugin/permissão não usado.
- [ ] Checklist de validação com hardware acima preenchida (resultados anotados neste arquivo ou no README do projeto).
- [ ] README do projeto com instruções de build/instalação/configuração de áudio.


> Nota do passo 13: `README.md` já existe na raiz (dev/testes, modo mini/Wayland, preferências) — complementar com empacotamento/instalação. Há um workaround de resize (reaplicar `set_size` após 150 ms em `window.rs`) validado só em X11. Capabilities atuais: `core:event:default` + `allow-*` dos commands próprios (sem fs/window/plugin).
