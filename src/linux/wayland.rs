use std::process::Command;
use crate::ffi::{Point, WindowSnapshot};
use super::DesktopBackend;

// ════════════════════════════════════════════════════════════════
// Hyprland  (hyprctl CLI protocol)
// ════════════════════════════════════════════════════════════════

pub struct HyprlandBackend;

impl HyprlandBackend {
    pub fn new() -> Self {
        log::info!("Hyprland backend initialised");
        Self
    }

    fn hyprctl(args: &[&str]) -> Option<String> {
        let output = Command::new("hyprctl").args(args).output().ok()?;
        if output.status.success() {
            String::from_utf8(output.stdout).ok()
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("hyprctl {:?} failed: {stderr}", args);
            None
        }
    }

    fn get_cursor_pos() -> Option<Point> {
        let raw = Self::hyprctl(&["cursorpos"])?.trim().to_string();
        let (x, y) = raw.split_once(',')?;
        Some(Point::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
    }

    fn get_active_window() -> Option<serde_json::Value> {
        let raw = Self::hyprctl(&["activewindow", "-j"])?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        if v.is_string() { return None; }
        Some(v)
    }

    fn parse_window_address(data: &serde_json::Value) -> Option<i64> {
        let addr = data.get("address").or(data.get("windowAddress"))?.as_str()?;
        if let Some(hex) = addr.strip_prefix("0x").or_else(|| addr.strip_prefix("0X")) {
            i64::from_str_radix(hex, 16).ok()
        } else {
            addr.parse().ok()
        }
    }
}

impl DesktopBackend for HyprlandBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let data = Self::get_active_window()?;
        let at = data.get("at")?.as_array()?;
        let size = data.get("size")?.as_array()?;
        if at.len() < 2 || size.len() < 2 { return None; }
        Some(WindowSnapshot {
            handle: Self::parse_window_address(&data).unwrap_or(0),
            position: Point::new(at[0].as_i64()? as i32, at[1].as_i64()? as i32),
            width: size[0].as_i64()? as i32,
            height: size[1].as_i64()? as i32,
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        if let Some(data) = Self::get_active_window() {
            if let Some(current) = Self::parse_window_address(&data) {
                if current != handle {
                    log::warn!("Hyprland active window changed during drag - aborting move");
                    return false;
                }
            }
            if let Some(at) = data.get("at").and_then(|v| v.as_array()) {
                if at.len() < 2 { return false; }
                let cx = at[0].as_i64().unwrap_or(0) as i32;
                let cy = at[1].as_i64().unwrap_or(0) as i32;
                let dx = target.x - cx;
                let dy = target.y - cy;
                if dx == 0 && dy == 0 { return true; }
                return Self::hyprctl(&["dispatch", "movewindowpixel", &format!("{} {}", dx, dy)]).is_some();
            }
        }
        false
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        if let Some(data) = Self::get_active_window() {
            if let Some(current) = Self::parse_window_address(&data) {
                return current == handle;
            }
        }
        false
    }

    fn current_cursor_position(&self) -> Option<Point> { Self::get_cursor_pos() }
    fn set_cursor_position(&self, point: Point) -> bool {
        Self::hyprctl(&["dispatch", "setcursorpos", &format!("{} {}", point.x, point.y)]).is_some()
    }
    fn mouse_button(&self, button: u8, press: bool) -> bool {
        let action = if press { "press" } else { "release" };
        Self::hyprctl(&["dispatch", "fakeinput", action, &button.to_string()]).is_some()
    }
    fn name(&self) -> &'static str { "Wayland / Hyprland" }
}

// ════════════════════════════════════════════════════════════════
// Sway  (swayipc crate)
// ════════════════════════════════════════════════════════════════

use swayipc as sway;

pub struct SwayBackend;

impl SwayBackend {
    pub fn new() -> Self {
        log::info!("Sway backend initialised");
        Self
    }

    fn connect() -> Option<sway::Connection> {
        sway::Connection::new().ok()
    }

    fn walk_tree<F>(node: &sway::Node, predicate: &F) -> Option<sway::Node>
    where F: Fn(&sway::Node) -> bool {
        if predicate(node) { return Some(node.clone()); }
        for child in &node.nodes {
            if let Some(found) = Self::walk_tree(child, predicate) { return Some(found); }
        }
        for child in &node.floating_nodes {
            if let Some(found) = Self::walk_tree(child, predicate) { return Some(found); }
        }
        None
    }

    fn focused_node(conn: &mut sway::Connection) -> Option<sway::Node> {
        let tree = conn.get_tree().ok()?;
        Self::walk_tree(&tree, &|n| n.focused)
    }

    fn node_exists(conn: &mut sway::Connection, id: i64) -> bool {
        conn.get_tree().ok()
            .and_then(|tree| Self::walk_tree(&tree, &|n| n.id as i64 == id))
            .is_some()
    }

    fn seat_cursor() -> Option<Point> {
        let mut conn = Self::connect()?;
        let seats = match conn.get_seats() { Ok(s) => s, Err(_) => return None };
        if !seats.is_empty() {
            log::warn!("Sway IPC does not expose cursor position; falling back to None");
        }
        None
    }
}

impl DesktopBackend for SwayBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let mut conn = Self::connect()?;
        let node = Self::focused_node(&mut conn)?;
        Some(WindowSnapshot {
            handle: node.id as i64,
            position: Point::new(node.rect.x as i32, node.rect.y as i32),
            width: node.rect.width as i32,
            height: node.rect.height as i32,
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        let mut conn = match Self::connect() { Some(c) => c, None => return false };
        if !Self::node_exists(&mut conn, handle) {
            log::warn!("Sway window handle {} no longer exists", handle);
            return false;
        }
        match conn.run_command(format!("move absolute position {} {}", target.x, target.y)) {
            Ok(results) => {
                for r in &results {
                    if r.is_err() {
                        if let Err(e) = r {
                            log::warn!("Sway move command error: {e}");
                        }
                    }
                }
                results.iter().any(|r| r.is_ok())
            }
            Err(_) => false,
        }
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        let mut conn = match Self::connect() { Some(c) => c, None => return false };
        Self::node_exists(&mut conn, handle)
    }

    fn current_cursor_position(&self) -> Option<Point> { Self::seat_cursor() }

    fn set_cursor_position(&self, point: Point) -> bool {
        let mut conn = match Self::connect() { Some(c) => c, None => return false };
        let seats = match conn.get_seats() { Ok(s) => s, Err(_) => return false };
        let name = match seats.first() { Some(s) => s.name.clone(), None => return false };
        match conn.run_command(format!("seat {} cursor set {} {}", name, point.x, point.y)) {
            Ok(results) => results.iter().any(|r| r.is_ok()),
            Err(_) => false,
        }
    }

    fn mouse_button(&self, _button: u8, _press: bool) -> bool {
        log::warn!("mouse click simulation not supported on Sway Wayland - use GestureAction::WindowMove instead");
        false
    }

    fn name(&self) -> &'static str { "Wayland / Sway" }
}

// ════════════════════════════════════════════════════════════════
// KDE KWin  (D-Bus + KWin Scripting API)
// ════════════════════════════════════════════════════════════════

pub struct KdeBackend;

impl KdeBackend {
    pub fn new() -> Self {
        log::info!("KDE KWin backend initialised");
        Self
    }

    fn run_script(js_body: &str) -> Option<String> {
        let tmp_dir = std::env::temp_dir().join("3-win-drag-kwin");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let script_path = tmp_dir.join("kwin_script.js");
        let output_path = tmp_dir.join("kwin_output.txt");

        let wrapped = format!(
            r#"var _3wd_out = new QFile("{}");
_3wd_out.open(QIODevice.WriteOnly | QIODevice.Truncate);
(function() {{ {} }})();
_3wd_out.close();"#,
            output_path.to_str()?,
            js_body
        );

        std::fs::write(&script_path, &wrapped).ok()?;
        let _ = std::fs::remove_file(&output_path);

        let script_str = script_path.to_str()?;
        let load_result = Command::new("qdbus")
            .args(["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript",
                   script_str, "3-win-drag-script"])
            .output()
            .or_else(|_| {
                let dbus_arg = format!("string:{}", script_str);
                Command::new("dbus-send")
                    .args(["--session", "--dest=org.kde.KWin", "--print-reply=literal",
                           "/Scripting", "org.kde.kwin.Scripting.loadScript",
                           &dbus_arg, "string:3-win-drag-script"])
                    .output()
            })
            .ok()
            .filter(|o| o.status.success())?;

        let script_id_str = String::from_utf8_lossy(&load_result.stdout)
            .lines().last().unwrap_or("").trim().to_string();

        let run_obj = format!("/Scripting/Script{}", script_id_str);
        let _ = Command::new("qdbus")
            .args(["org.kde.KWin", &run_obj, "org.kde.kwin.Script.run"])
            .output()
            .or_else(|_| {
                Command::new("dbus-send")
                    .args(["--session", "--dest=org.kde.KWin", &run_obj, "org.kde.kwin.Script.run"])
                    .output()
            });

        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::read_to_string(&output_path).ok()
    }
}

impl DesktopBackend for KdeBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let script = r#"var client = workspace.activeClient;
if (client) {
    var geo = client.geometry;
    _3wd_out.write(geo.x + "," + geo.y + "," + geo.width + "," + geo.height);
} else { _3wd_out.write("none"); }"#;
        let result = Self::run_script(script)?;
        let trimmed = result.trim().to_string();
        if trimmed == "none" { return None; }
        let parts: Vec<i32> = trimmed.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() < 4 { return None; }
        Some(WindowSnapshot {
            handle: 0,
            position: Point::new(parts[0], parts[1]),
            width: parts[2], height: parts[3],
            was_maximized: false,
        })
    }

    fn move_window(&self, _handle: i64, target: Point) -> bool {
        let script = format!(
            "var client = workspace.activeClient;\nif (client) {{\n    client.geometry = {{ x: {}, y: {}, width: client.geometry.width, height: client.geometry.height }};\n}}",
            target.x, target.y
        );
        Self::run_script(&script).is_some()
    }

    fn window_is_valid(&self, _handle: i64) -> bool {
        let script = r#"var client = workspace.activeClient; _3wd_out.write(client ? "valid" : "invalid");"#;
        Self::run_script(script).unwrap_or_default().trim() == "valid"
    }

    fn current_cursor_position(&self) -> Option<Point> {
        let script = r#"var pos = workspace.cursorPos; _3wd_out.write(pos.x + "," + pos.y);"#;
        let result = Self::run_script(script)?;
        let parts: Vec<i32> = result.trim().split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() < 2 { return None; }
        Some(Point::new(parts[0], parts[1]))
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        let script = format!("workspace.cursorPos = {{ x: {}, y: {} }};", point.x, point.y);
        Self::run_script(&script).is_some()
    }

    fn mouse_button(&self, _button: u8, _press: bool) -> bool {
        log::warn!("mouse click simulation not supported on KDE Wayland - use GestureAction::WindowMove instead");
        false
    }

    fn name(&self) -> &'static str { "Wayland / KDE Plasma (KWin)" }
}
