// ═══════════════════════════════════════════════════════════════════════
// Linux Backend — Runtime display-server detection + compositor dispatch
// ═══════════════════════════════════════════════════════════════════════

mod x11;
mod wayland;

use crate::ffi::{Point, WindowSnapshot};

/// Unified desktop-backend abstraction.  Selected once at startup based on
/// `$XDG_SESSION_TYPE` and compositor-specific environment variables.
pub trait DesktopBackend: Send {
    fn prepare_foreground_window(&self, cursor: Point) -> Option<WindowSnapshot>;
    fn move_window(&self, handle: i64, target: Point) -> bool;
    fn window_is_valid(&self, handle: i64) -> bool;
    fn current_cursor_position(&self) -> Option<Point>;
    fn set_cursor_position(&self, point: Point) -> bool;
    fn mouse_button(&self, button: u8, press: bool) -> bool;
    fn name(&self) -> &'static str;
}

// ──────────────── Detection ────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopKind {
    X11,
    WaylandHyprland,
    WaylandSway,
    WaylandKde,
    Unsupported,
}

pub fn detect() -> DesktopKind {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let is_wayland = session_type == "wayland"
        || std::env::var("WAYLAND_DISPLAY").is_ok();

    if is_wayland {
        // Hyprland — most popular standalone Wayland compositor
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            return DesktopKind::WaylandHyprland;
        }
        // Sway — i3-compatible wlroots compositor
        if std::env::var("SWAYSOCK").is_ok() {
            return DesktopKind::WaylandSway;
        }
        // KDE / KWin — most popular full DE on Wayland
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_default()
            .to_lowercase();
        if desktop.contains("kde") || desktop.contains("plasma") {
            return DesktopKind::WaylandKde;
        }
        // GNOME / unknown — window management is not possible on GNOME Wayland
        log::warn!(
            "Wayland detected but compositor '{}' is not supported for window management. \
             Only Hyprland, Sway, and KDE are supported on Wayland.\n\
             Falling back to XWayland (X11) — window positions may be inaccurate for native \
             Wayland clients.",
            desktop
        );
        DesktopKind::Unsupported
    } else {
        DesktopKind::X11
    }
}

pub fn backend_name(kind: DesktopKind) -> &'static str {
    match kind {
        DesktopKind::X11 => "X11 (x11rb)",
        DesktopKind::WaylandHyprland => "Wayland / Hyprland",
        DesktopKind::WaylandSway => "Wayland / Sway",
        DesktopKind::WaylandKde => "Wayland / KDE Plasma",
        DesktopKind::Unsupported => "unsupported",
    }
}

// ──────────────── Singleton backend ────────────────

static BACKEND: std::sync::OnceLock<Box<dyn DesktopBackend + Sync>> = std::sync::OnceLock::new();

fn backend() -> &'static dyn DesktopBackend {
    BACKEND.get_or_init(|| {
        let kind = detect();
        log::info!("Linux desktop backend: {}", backend_name(kind));

        match kind {
            DesktopKind::X11 => {
                match x11::X11Backend::new() {
                    Ok(b) => Box::new(b),
                    Err(e) => {
                        log::error!("failed to initialise X11 backend: {e:#}");
                        Box::new(NullBackend)
                    }
                }
            }
            DesktopKind::WaylandHyprland => Box::new(wayland::HyprlandBackend::new()),
            DesktopKind::WaylandSway => Box::new(wayland::SwayBackend::new()),
            DesktopKind::WaylandKde => Box::new(wayland::KdeBackend::new()),
            DesktopKind::Unsupported => {
                log::error!(
                    "Unsupported Wayland compositor — window management is disabled.\n\
                     Please use an X11 session or one of the supported compositors: \
                     Hyprland, Sway, KDE Plasma."
                );
                Box::new(NullBackend)
            }
        }
    }).as_ref()
}

// ──────────────── Null backend (graceful fallback) ────────────────

struct NullBackend;

impl DesktopBackend for NullBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> { None }
    fn move_window(&self, _handle: i64, _target: Point) -> bool { false }
    fn window_is_valid(&self, _handle: i64) -> bool { false }
    fn current_cursor_position(&self) -> Option<Point> { None }
    fn set_cursor_position(&self, _point: Point) -> bool { false }
    fn mouse_button(&self, _button: u8, _press: bool) -> bool { false }
    fn name(&self) -> &'static str { "null (unsupported compositor)" }
}

// ──────────────── Public API called from ffi.rs ────────────────

pub fn prepare_foreground_window(cursor: Point) -> Option<WindowSnapshot> {
    backend().prepare_foreground_window(cursor)
}

pub fn move_window(handle: i64, target: Point) -> bool {
    backend().move_window(handle, target)
}

pub fn window_is_valid(handle: i64) -> bool {
    backend().window_is_valid(handle)
}

pub fn current_cursor_position() -> Option<Point> {
    backend().current_cursor_position()
}

pub fn set_cursor_position(point: Point) -> bool {
    backend().set_cursor_position(point)
}

pub fn mouse_button(button: u8, press: bool) -> bool {
    backend().mouse_button(button, press)
}

pub fn log_backend_info() {
    let backend = backend();
    log::info!("active Linux backend: {}", backend.name());
}
