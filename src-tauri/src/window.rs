//! Modo mini: janela compacta, sempre no topo, controlada pelo Rust
//! (evita conceder permissoes `core:window:*` ao frontend).

use serde::Serialize;
use std::time::Duration;

use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, Size, WebviewWindow};

/// Tamanho do modo mini (logico).
pub const MINI_SIZE: (f64, f64) = (280.0, 160.0);
/// Tamanho minimo do modo normal (igual a `tauri.conf.json`).
pub const NORMAL_MIN_SIZE: (f64, f64) = (360.0, 560.0);
/// Tamanho padrao do modo normal (igual a `tauri.conf.json`).
pub const NORMAL_DEFAULT_SIZE: (f64, f64) = (480.0, 720.0);

/// Espera antes de reaplicar o tamanho ao sair do mini (ver `apply_mini`).
const RESIZE_RETRY_DELAY: Duration = Duration::from_millis(150);

/// Estado da janela exposto ao frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WindowState {
    pub mini_mode: bool,
    /// Sessao Wayland: o compositor pode ignorar "sempre no topo".
    pub wayland: bool,
}

/// Geometria a restaurar ao sair do modo mini.
#[derive(Debug, Clone, Copy)]
pub struct SavedGeometry {
    pub size: PhysicalSize<u32>,
    pub position: Option<PhysicalPosition<i32>>,
}

/// Detecta Wayland a partir das variaveis de ambiente (`get` injetavel para testes).
/// `GDK_BACKEND=x11` forca XWayland, onde "sempre no topo" funciona.
pub fn is_wayland_session(get: impl Fn(&str) -> Option<String>) -> bool {
    let forces_x11 = get("GDK_BACKEND")
        .and_then(|v| v.split(',').next().map(|b| b.trim().eq_ignore_ascii_case("x11")))
        .unwrap_or(false);
    if forces_x11 {
        return false;
    }
    get("XDG_SESSION_TYPE").is_some_and(|v| v.eq_ignore_ascii_case("wayland"))
        || get("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

pub fn wayland_from_env() -> bool {
    is_wayland_session(|k| std::env::var(k).ok())
}

/// Entra/sai do modo mini. `saved` guarda a geometria normal enquanto estiver no mini.
pub fn apply_mini(
    window: &WebviewWindow,
    enable: bool,
    saved: &mut Option<SavedGeometry>,
) -> tauri::Result<()> {
    if enable {
        if saved.is_none() {
            let geometry = SavedGeometry {
                size: window.inner_size()?,
                position: window.outer_position().ok(),
            };
            log::info!("modo mini: guardando geometria {geometry:?}");
            *saved = Some(geometry);
        }
        // O min_size normal (360x560) impediria encolher: reduz primeiro.
        window.set_min_size(Some(LogicalSize::new(MINI_SIZE.0, MINI_SIZE.1)))?;
        window.set_size(LogicalSize::new(MINI_SIZE.0, MINI_SIZE.1))?;
        window.set_always_on_top(true)?;
    } else {
        window.set_always_on_top(false)?;
        let (size, position): (Size, Option<PhysicalPosition<i32>>) = match saved.take() {
            Some(g) => {
                log::info!("modo mini: restaurando geometria {g:?}");
                (g.size.into(), g.position)
            }
            None => (LogicalSize::new(NORMAL_DEFAULT_SIZE.0, NORMAL_DEFAULT_SIZE.1).into(), None),
        };
        window.set_min_size(Some(LogicalSize::new(NORMAL_MIN_SIZE.0, NORMAL_MIN_SIZE.1)))?;
        window.set_size(size)?;
        if let Some(position) = position
            && let Err(e) = window.set_position(position)
        {
            log::debug!("nao foi possivel restaurar a posicao: {e}");
        }
        // Mudar o min_size faz o GTK/WM redimensionar a janela para o minimo e sobrescrever
        // o `set_size` acima (ficava em 360x560). Reaplica o tamanho logo depois.
        let window = window.clone();
        let spawned = std::thread::Builder::new().name("groove-window".into()).spawn(move || {
            std::thread::sleep(RESIZE_RETRY_DELAY);
            if let Err(e) = window.set_size(size) {
                log::debug!("segundo set_size falhou: {e}");
            }
        });
        if let Err(e) = spawned {
            log::debug!("nao foi possivel agendar o segundo set_size: {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn detects_wayland() {
        assert!(is_wayland_session(env(&[("XDG_SESSION_TYPE", "wayland")])));
        assert!(is_wayland_session(env(&[("WAYLAND_DISPLAY", "wayland-0")])));
        assert!(!is_wayland_session(env(&[("XDG_SESSION_TYPE", "x11")])));
        assert!(!is_wayland_session(env(&[("WAYLAND_DISPLAY", "")])));
        assert!(!is_wayland_session(env(&[])));
    }

    #[test]
    fn gdk_backend_x11_overrides_wayland() {
        let e = env(&[("XDG_SESSION_TYPE", "wayland"), ("WAYLAND_DISPLAY", "wayland-0"), ("GDK_BACKEND", "x11")]);
        assert!(!is_wayland_session(e));
        let e = env(&[("XDG_SESSION_TYPE", "wayland"), ("GDK_BACKEND", "x11,wayland")]);
        assert!(!is_wayland_session(e));
        let e = env(&[("XDG_SESSION_TYPE", "wayland"), ("GDK_BACKEND", "wayland,x11")]);
        assert!(is_wayland_session(e));
    }
}
