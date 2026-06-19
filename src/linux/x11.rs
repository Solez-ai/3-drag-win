use std::sync::{Mutex, OnceLock};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{self, ConnectionExt as _, AtomEnum, ClientMessageEvent};
use x11rb::rust_connection::RustConnection;
use crate::ffi::{Point, WindowSnapshot};
use super::DesktopBackend;

pub struct X11Backend;

impl X11Backend {
    pub fn new() -> Result<Self, String> {
        let _guard = get_connection().lock()
            .map_err(|e| format!("failed to acquire X11 connection lock: {e}"))?;
        Ok(Self)
    }
}

impl DesktopBackend for X11Backend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let conn = get_connection().lock().ok()?;
        let screen = &conn.setup().roots[0];
        let window = get_active_window(&*conn, screen.root)?;

        let attrs = conn.get_window_attributes(window).ok()?.reply().ok()?;
        if attrs.map_state == xproto::MapState::UNMAPPED
            || attrs.map_state == xproto::MapState::UNVIEWABLE
        {
            return None;
        }

        let fullscreen_atom = conn.intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")
            .ok()?.reply().ok()?.atom;
        let wm_state_atom = conn.intern_atom(false, b"_NET_WM_STATE")
            .ok()?.reply().ok()?.atom;
        if let Ok(state_reply) = conn.get_property(
            false, window, wm_state_atom, AtomEnum::ATOM, 0, 1024,
        ).ok()?.reply() {
            if state_reply.format == 32 {
                for chunk in state_reply.value.chunks_exact(4) {
                    let atom_val = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    if atom_val == fullscreen_atom { return None; }
                }
            }
        }

        let geom = conn.get_geometry(window).ok()?.reply().ok()?;
        let translation = conn.translate_coordinates(window, screen.root, 0, 0)
            .ok()?.reply().ok()?;

        if translation.dst_x <= 0 && translation.dst_y <= 0
            && (geom.width as i16) >= screen.width_in_pixels as i16
            && (geom.height as i16) >= screen.height_in_pixels as i16
        {
            return None;
        }

        Some(WindowSnapshot {
            handle: window as i64,
            position: Point::new(translation.dst_x as i32, translation.dst_y as i32),
            width: geom.width as i32,
            height: geom.height as i32,
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        let conn = match get_connection().lock() { Ok(c) => c, Err(_) => return false };
        let window = handle as u32;
        let screen = &conn.setup().roots[0];

        let net_atom = conn.intern_atom(false, b"_NET_MOVE_WINDOW").ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom);

        if let Some(atom) = net_atom {
            let data: [u32; 5] = [0, target.x as u32, target.y as u32, 0, 0];
            let event = ClientMessageEvent::new(32, window, atom, data);
            if conn.send_event(true, screen.root, xproto::EventMask::NO_EVENT, &event).is_ok() {
                let _ = conn.flush();
                return true;
            }
            false
        } else {
            if conn.configure_window(window, &xproto::ConfigureWindowAux::new().x(target.x).y(target.y)).is_ok() {
                let _ = conn.flush();
                return true;
            }
            false
        }
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        if let Ok(conn) = get_connection().lock() {
            conn.get_window_attributes(handle as u32).is_ok()
        } else {
            false
        }
    }

    fn current_cursor_position(&self) -> Option<Point> {
        let conn = get_connection().lock().ok()?;
        let screen = &conn.setup().roots[0];
        let reply = conn.query_pointer(screen.root).ok()?.reply().ok()?;
        Some(Point::new(reply.root_x as i32, reply.root_y as i32))
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        if let Ok(conn) = get_connection().lock() {
            let screen = &conn.setup().roots[0];
            conn.warp_pointer(screen.root, 0u32, 0_i16, 0_i16, 0u16, 0u16, point.x as i16, point.y as i16).is_ok()
        } else {
            false
        }
    }

    fn mouse_button(&self, button: u8, press: bool) -> bool {
        if let Ok(conn) = get_connection().lock() {
            let screen = &conn.setup().roots[0];
            let action: u8 = if press { 4 } else { 5 };
            x11rb::protocol::xtest::fake_input(&*conn, action, button, 0, screen.root, 0, 0, 0).is_ok()
        } else {
            false
        }
    }

    fn name(&self) -> &'static str { "X11 (x11rb)" }
}

fn get_connection() -> &'static Mutex<RustConnection> {
    static X11_CONN: OnceLock<Mutex<RustConnection>> = OnceLock::new();
    X11_CONN.get_or_init(|| {
        let (conn, _) = x11rb::connect(None).expect("failed to connect to X11 display");
        Mutex::new(conn)
    })
}

fn get_active_window(conn: &RustConnection, screen_root: u32) -> Option<u32> {
    let atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
    let reply = conn.get_property(false, screen_root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?.reply().ok()?;
    if reply.format != 32 || reply.value.len() < 4 { return None; }
    let window = u32::from_ne_bytes([reply.value[0], reply.value[1], reply.value[2], reply.value[3]]);
    if window == 0 { None } else { Some(window) }
}
