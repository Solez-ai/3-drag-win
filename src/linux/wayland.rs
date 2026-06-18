// ═══════════════════════════════════════════════════════════════════════
// Wayland Backends — compositor-specific window management
//
// Wayland does not allow arbitrary clients to move other clients' windows
// by design.  Each compositor provides a bespoke IPC mechanism instead:
//
//   Hyprland   — Unix socket + hyprctl CLI (hyprland-rs crate also available)
//   Sway       — i3-compatible IPC via swayipc crate
//   KDE Plasma — D-Bus KWin Scripting API via zbus / qdbus
// ═══════════════════════════════════════════════════════════════════════

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

    /// Run a `hyprctl` command and return stdout.
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

    /// Parse `hyprctl cursorpos` — returns `"x, y"` string.
    fn get_cursor_pos() -> Option<Point> {
        let raw = Self::hyprctl(&["cursorpos"])?.trim().to_string();
        let (x, y) = raw.split_once(',')?;
        Some(Point::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
    }

    /// Parse `hyprctl activewindow -j` for active window data.
    fn get_active_window() -> Option<serde_json::Value> {
        let raw = Self::hyprctl(&["activewindow", "-j"])?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        if v.is_string() {
            return None; // "not found"
        }
        Some(v)
    }

    /// Parse the Hyprland window address string (e.g. "0x55a1234") as i64.
    fn parse_window_address(data: &serde_json::Value) -> Option<i64> {
        let addr = data.get("address")?
            .or_else(|| data.get("windowAddress"))
            .and_then(|v| v.as_str())?;
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
        let at = data.get("at")?;
        let size = data.get("size")?;
        let at_arr: Vec<i32> = at.as_array()?
            .iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect();
        let size_arr: Vec<i32> = size.as_array()?
            .iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect();
        if at_arr.len() < 2 || size_arr.len() < 2 {
            return None;
        }

        Some(WindowSnapshot {
            handle: Self::parse_window_address(&data).unwrap_or(0),
            position: Point::new(at_arr[0], at_arr[1]),
            width: size_arr[0],
            height: size_arr[1],
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        // Verify the window hasn't changed mid-drag; abort if so
        if let Some(data) = Self::get_active_window() {
            if let Some(current) = Self::parse_window_address(&data) {
                if current != handle {
                    log::warn!("Hyprland active window changed during drag — aborting move");
                    return false;
                }
            }
            // Compute delta from current position
            if let Some(at) = data.get("at").and_then(|v| v.as_array()) {
                let cx = at.first()?.as_i64()? as i32;
                let cy = at.get(1)?.as_i64()? as i32;
                let dx = target.x - cx;
                let dy = target.y - cy;
                if dx == 0 && dy == 0 {
                    return true;
                }
                return Self::hyprctl(&["dispatch", "movewindowpixel", &format!("{} {}", dx, dy)]).is_some();
            }
        }
        false
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        let data = Self::get_active_window()?;
        let current = Self::parse_window_address(&data)?;
        Some(current == handle)
    }

    fn current_cursor_position(&self) -> Option<Point> {
        Self::get_cursor_pos()
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        Self::hyprctl(&["dispatch", "setcursorpos", &format!("{} {}", point.x, point.y)]).is_some()
    }

    fn mouse_button(&self, button: u8, press: bool) -> bool {
        let action = if press { "press" } else { "release" };
        // hyprctl dispatch fakeinput <action> <btn>
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
        sway::Connection::new()
            .map_err(|e| {
                log::error!("failed to connect to Sway IPC socket: {e}");
                e
            })
            .ok()
    }

    /// Walk the Sway tree to find a node by predicate.
    fn walk_tree<F>(node: &sway::Node, predicate: &F) -> Option<sway::Node>
    where
        F: Fn(&sway::Node) -> bool,
    {
        if predicate(node) {
            return Some(node.clone());
        }
        for child in &node.nodes {
            if let Some(found) = Self::walk_tree(child, predicate) {
                return Some(found);
            }
        }
        for child in &node.floating_nodes {
            if let Some(found) = Self::walk_tree(child, predicate) {
                return Some(found);
            }
        }
        None
    }

    /// Find the focused window node by walking the Sway tree.
    fn focused_node(conn: &sway::Connection) -> Option<sway::Node> {
        let tree = conn.get_tree().ok()?;
        Self::walk_tree(&tree, &|n| n.focused)
    }

    /// Check if a node with the given id exists anywhere in the tree.
    fn node_exists(conn: &sway::Connection, id: i64) -> bool {
        conn.get_tree()
            .ok()
            .and_then(|tree| Self::walk_tree(&tree, &|n| n.id as i64 == id))
            .is_some()
    }

    /// Get cursor position from the Sway seat.
    fn seat_cursor(conn: &sway::Connection) -> Option<Point> {
        let seats = conn.get_seats().ok()?;
        for seat in &seats {
            // seat.cursor is Option<Cursor> with position fields
            // In swayipc 4.x, the Seat struct has `cursor` field with `Cursor`:
            // pub struct Cursor { pub position: (i64, i64, bool, bool) }
            if let Some(ref cursor) = seat.cursor {
                let (x, y, _, _) = cursor.position;
                return Some(Point::new(x as i32, y as i32));
            }
        }
        None
    }
}

impl DesktopBackend for SwayBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let conn = Self::connect()?;
        let node = Self::focused_node(&conn)?;
        let id = node.id as i64;
        let r = &node.rect;
        Some(WindowSnapshot {
            handle: id,
            position: Point::new(r.x as i32, r.y as i32),
            width: r.width as i32,
            height: r.height as i32,
            was_maximized: false,
        })
    }

    fn move_window(&self, handle: i64, target: Point) -> bool {
        let mut conn = match Self::connect() {
            Some(c) => c,
            None => return false,
        };
        if !Self::node_exists(&conn, handle) {
            log::warn!("Sway window handle {} no longer exists", handle);
            return false;
        }
        conn.run_command(format!("move absolute position {} {}", target.x, target.y))
            .ok()
            .map(|results| {
                for r in &results {
                    if let Some(err) = &r.error {
                        log::warn!("Sway move command error: {err}");
                    }
                }
                results.iter().any(|r| r.success)
            })
            .unwrap_or(false)
    }

    fn window_is_valid(&self, handle: i64) -> bool {
        let conn = match Self::connect() {
            Some(c) => c,
            None => return false,
        };
        Self::node_exists(&conn, handle)
    }

    fn current_cursor_position(&self) -> Option<Point> {
        let conn = Self::connect()?;
        Self::seat_cursor(&conn)
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        let mut conn = match Self::connect() {
            Some(c) => c,
            None => return false,
        };
        // Get first seat name
        let seats = conn.get_seats().ok()?;
        let name = &seats.first()?.name;
        conn.run_command(format!("seat {} cursor set {} {}", name, point.x, point.y))
            .ok()
            .map(|results| results.iter().any(|r| r.success))
            .unwrap_or(false)
    }

    fn mouse_button(&self, _button: u8, _press: bool) -> bool {
        log::warn!("mouse click simulation not supported on Sway Wayland — use GestureAction::WindowMove instead");
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

    /// Write a KWin JavaScript snippet to a temp file, load it via D-Bus,
    /// run it, and read back the output from a known temp file.
    fn run_script(js_body: &str) -> Option<String> {
        let tmp_dir = std::env::temp_dir().join("3-win-drag-kwin");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let script_path = tmp_dir.join("kwin_script.js");
        let output_path = tmp_dir.join("kwin_output.txt");

        // Wrap the JS to write results to a file
        let wrapped = format!(
            r#"
var _3wd_out = new QFile("{}");
_3wd_out.open(QIODevice.WriteOnly | QIODevice.Truncate);
(function() {{
    {body}
}})();
_3wd_out.close();
"#,
            output_path.to_str()?,
            body = js_body
        );

        std::fs::write(&script_path, &wrapped).ok()?;
        // Remove stale output
        let _ = std::fs::remove_file(&output_path);

        // Load script via qdbus (preferred) or dbus-send
        let load_result = Command::new("qdbus")
            .args([
                "org.kde.KWin",
                "/Scripting",
                "org.kde.kwin.Scripting.loadScript",
                script_path.to_str()?,
                "3-win-drag-script",
            ])
            .output()
            .or_else(|_| {
                // Fallback: call via dbus-send
                Command::new("dbus-send")
                    .args([
                        "--session",
                        "--dest=org.kde.KWin",
                        "--print-reply=literal",
                        "/Scripting",
                        "org.kde.kwin.Scripting.loadScript",
                        &format!("string:{}", script_path.to_str()?),
                        "string:3-win-drag-script",
                    ])
                    .output()
            })
            .ok()
            .filter(|o| o.status.success())?;

        let script_id_str = String::from_utf8_lossy(&load_result.stdout)
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .to_string();

        // Run the loaded script
        let run_obj = format!("/Scripting/Script{}", script_id_str);
        Command::new("qdbus")
            .args(["org.kde.KWin", &run_obj, "org.kde.kwin.Script.run"])
            .output()
            .or_else(|_| {
                Command::new("dbus-send")
                    .args(["--session", "--dest=org.kde.KWin", &run_obj, "org.kde.kwin.Script.run"])
                    .output()
            })
            .ok()?;

        // Give KWin a moment to execute and flush
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Read the output file
        std::fs::read_to_string(&output_path).ok()
    }
}

impl DesktopBackend for KdeBackend {
    fn prepare_foreground_window(&self, _cursor: Point) -> Option<WindowSnapshot> {
        let script = r#"
var client = workspace.activeClient;
if (client) {
    var geo = client.geometry;
    _3wd_out.write(geo.x + "," + geo.y + "," + geo.width + "," + geo.height);
} else {
    _3wd_out.write("none");
}
"#;
        let result = Self::run_script(script)?;
        let trimmed = result.trim().to_string();
        if trimmed == "none" {
            return None;
        }
        let parts: Vec<i32> = trimmed.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() < 4 {
            return None;
        }
        // Use client.internalId as handle if available (numeric string)
        let handle = 0i64; // KWin doesn't give easy numeric IDs via simple JS
        Some(WindowSnapshot {
            handle,
            position: Point::new(parts[0], parts[1]),
            width: parts[2],
            height: parts[3],
            was_maximized: false,
        })
    }

    fn move_window(&self, _handle: i64, target: Point) -> bool {
        let script = format!(
r#"var client = workspace.activeClient;
if (client) {{
    var geo = client.geometry;
    client.geometry = {{ x: {}, y: {}, width: geo.width, height: geo.height }};
}}"#,
            target.x, target.y
        );
        Self::run_script(&script).is_some()
    }

    fn window_is_valid(&self, _handle: i64) -> bool {
        let script = r#"
var client = workspace.activeClient;
_3wd_out.write(client ? "valid" : "invalid");
"#;
        let result = Self::run_script(script).unwrap_or_default();
        result.trim() == "valid"
    }

    fn current_cursor_position(&self) -> Option<Point> {
        let script = r#"
var pos = workspace.cursorPos;
_3wd_out.write(pos.x + "," + pos.y);
"#;
        let result = Self::run_script(script)?;
        let parts: Vec<i32> = result.trim().split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if parts.len() < 2 { return None; }
        Some(Point::new(parts[0], parts[1]))
    }

    fn set_cursor_position(&self, point: Point) -> bool {
        let script = format!(
r#"workspace.cursorPos = {{ x: {}, y: {} }};"#,
            point.x, point.y
        );
        Self::run_script(&script).is_some()
    }

    fn mouse_button(&self, _button: u8, _press: bool) -> bool {
        log::warn!("mouse click simulation not supported on KDE Wayland — use GestureAction::WindowMove instead");
        false
    }

    fn name(&self) -> &'static str { "Wayland / KDE Plasma (KWin)" }
}
