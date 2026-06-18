// ═══════════════════════════════════════════════════════════════════════
// X11 Backend - x11rb pure-Rust X11 protocol implementation
// ═══════════════════════════════════════════════════════════════════════

use std::sync::{Mutex, OnceLock};

use x11rb::connection::Connection;
use x11rb::protocol::xproto;
use x11rb::rust_connection::RustConnection;

use crate::ffi::{Point, WindowSnapshot};
use super::DesktopBackend;

pub struct X11Backend {
    // Connection is held in a Mutex so it can be shared across threads safely
}

impl X11Backend {
    pub fn new() -> Result<Self, String> {
        // Force connection init so failures surface during construction
        let _ = get_connection()
            .lock()
            .map_err(|e| format!("failed to acquire X11 connection lock: {e}"))?;
        Ok(Self)
    }
}

impl DesktopBackend for X11Backend {
    fn prepare_foreground_window(&self, cursor: Point) -> Option<WindowSnapshot> {
        let conn = get_connection().lock().ok()?;
        let screen = &conn.setup().roots[0];
        let window = get_active_window(&*conn, screen.root)?;

        // Reject unmapped / iconic windows
        let attrs = conn.get_window_attributes(window).ok()?;
        match attrs.map_state {
            xproto::MapState::UNMAPPED | xproto::MapState::UNVIEWABLE => return None,
            _ => {}
        }

        // Check _NET_WM_STATE for fullscreen
        let fullscreen_atom = conn
            .intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")
            .ok()
            .map(|r| r.atom);
        if let Some(fs_atom) = fullscreen_atom {
            let wm_state_atom = conn.intern_atom(false, b"_NET_WM_STATE").ok()?.atom;
            if let Ok(state_reply) = conn.get_property(
                false, window, wm_state_atom, xproto::ATOM_ATOM, 0, 1024,
            ) {
                if state_reply.format == 32 {
                    for chunk in state_reply.value.chunks_exact(4) {
                        let atom_val =
                            u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        if atom_val == fs_atom {
                            return None;
                        }
                    }
                }
            }
        }

        // Geometric fullscreen check
        let geom = conn.get_geometry(window).ok()?;
        let translation = conn.translate_coordinates(window, screen.root, 0, 0).ok()?;

        let covers_screen = translation.dst_x <= 0
            && translation.dst_y <= 0
            && (geom.width as i16) >= screen.width_in_pixels as i16
            && (geom.height as i16) >= screen.height_in_pixels as i16;
        if covers_screen {
            return None;
        }

        Some(WindowSnapshot {
            handle: window as i64,
            position: Point::new(translation.dst_x, translation.dst_y),
            width: geom.width as i32,
            height: geom.height as i32,
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        let conn = match get_connection().lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let window = handle as u32;
        let screen = &conn.setup().roots[0];

        // Prefer _NET_MOVE_WINDOW client message (EWMH standard)
        let net_move_window = conn
            .intern_atom(false, b"_NET_MOVE_WINDOW")
            .ok()
            .map(|r| r.atom);

        if let Some(atom) = net_move_window {
            let data: [u32; 5] = [0, target.x as u32, target.y as u32, 0, 0];
            let event = xproto::ClientMessageEvent::new_unchecked(window, atom, 32, data);
            conn.send_event(
                true,
                screen.root,
                0xFFu32,
                &xproto::Event::ClientMessage(event),
            )
            .ok()?;
            conn.flush().ok()?;
            true
        } else {
            conn.configure_window(
                window,
                &[xproto::ConfigWindow::X(target.x), xproto::ConfigWindow::Y(target.y)],
            )
            .ok()?;
            conn.flush().ok()?;
            true
        }
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        let conn = match get_connection().lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.get_window_attributes(handle as u32).is_ok()
    }

    fn current_cursor_position(&self) -> Option<Point> {
        let conn = get_connection().lock().ok()?;
        let screen = &conn.setup().roots[0];
        let reply = conn.query_pointer(screen.root).ok()?;
        Some(Point::new(reply.root_x, reply.root_y))
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        let conn = match get_connection().lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let screen = &conn.setup().roots[0];
        conn.warp_pointer(screen.root, 0, 0, 0, 0, 0, point.x, point.y)
            .ok()?;
        conn.flush().ok()?;
        true
    }

    fn mouse_button(&self, button: u8, press: bool) -> bool {
        let conn = match get_connection().lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let screen = &conn.setup().roots[0];
        let action = if press {
            x11rb::protocol::xtest::FakeInput::BUTTON_PRESS
        } else {
            x11rb::protocol::xtest::FakeInput::BUTTON_RELEASE
        };
        x11rb::protocol::xtest::fake_input(
            &*conn, action, button, 0, screen.root, 0, 0,
        )
        .ok()?;
        conn.flush().ok()?;
        true
    }

    fn name(&self) -> &'static str {
        "X11 (x11rb)"
    }
}

// ──────────────── Shared X11 connection ────────────────

fn get_connection() -> &'static Mutex<RustConnection> {
    static X11_CONN: OnceLock<Mutex<RustConnection>> = OnceLock::new();
    X11_CONN.get_or_init(|| {
        let (conn, _) = x11rb::connect(None).expect("failed to connect to X11 display");
        Mutex::new(conn)
    })
}

fn get_active_window(conn: &RustConnection, screen_root: u32) -> Option<u32> {
    let atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.atom;
    let reply = conn
        .get_property(false, screen_root, atom, xproto::ATOM_WINDOW, 0, 1)
        .ok()?;
    if reply.format != 32 || reply.value.len() < 4 {
        return None;
    }
    let window = u32::from_ne_bytes([reply.value[0], reply.value[1], reply.value[2], reply.value[3]]);
    if window == 0 { None } else { Some(window) }
}
