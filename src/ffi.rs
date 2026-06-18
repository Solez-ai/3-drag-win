#[cfg(all(windows, not(has_cpp_backend)))]
use std::time::Duration;

#[cfg(target_os = "linux")]
use crate::linux as linux_backend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WindowSnapshot {
    pub handle: i64,
    pub position: Point,
    pub width: i32,
    pub height: i32,
    pub was_maximized: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct NativeWindowState {
    handle: i64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    was_maximized: i32,
}

#[cfg(all(windows, has_cpp_backend))]
unsafe extern "C" {
    fn drag_bootstrap_process() -> i32;
    fn drag_prepare_foreground_window(
        cursor_x: i32,
        cursor_y: i32,
        out_state: *mut NativeWindowState,
    ) -> i32;
    fn drag_move_window(handle: i64, x: i32, y: i32) -> i32;
    fn drag_window_is_valid(handle: i64) -> i32;
    fn drag_get_cursor_position(x: *mut i32, y: *mut i32) -> i32;
}

// ──────────────── Bootstrap ────────────────

pub fn bootstrap_process() {
    #[cfg(all(windows, has_cpp_backend))]
    unsafe {
        let _ = drag_bootstrap_process();
    }

    #[cfg(all(windows, not(has_cpp_backend)))]
    unsafe {
        winapi::um::winuser::SetProcessDPIAware();
    }

    #[cfg(target_os = "linux")]
    {
        // Initialise the chosen backend (X11/Wayland) - logs which one was detected
        linux_backend::log_backend_info();
    }
}

// ──────────────── Foreground Window ────────────────

pub fn prepare_foreground_window(cursor: Point) -> Option<WindowSnapshot> {
    #[cfg(all(windows, has_cpp_backend))]
    unsafe {
        let mut state = NativeWindowState {
            handle: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            was_maximized: 0,
        };
        if drag_prepare_foreground_window(cursor.x, cursor.y, &mut state as *mut _) == 0 {
            return None;
        }
        return Some(WindowSnapshot {
            handle: state.handle,
            position: Point::new(state.x, state.y),
            width: state.width,
            height: state.height,
            was_maximized: state.was_maximized != 0,
        });
    }

    #[cfg(all(windows, not(has_cpp_backend)))]
    unsafe {
        return prepare_foreground_window_fallback(cursor);
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::prepare_foreground_window(cursor);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = cursor;
        None
    }
}

// ──────────────── Move Window ────────────────

pub fn move_window(handle: i64, target: Point) -> bool {
    #[cfg(all(windows, has_cpp_backend))]
    unsafe {
        return drag_move_window(handle, target.x, target.y) != 0;
    }

    #[cfg(all(windows, not(has_cpp_backend)))]
    unsafe {
        use winapi::shared::windef::HWND;
        use winapi::um::winuser::{
            SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
        };

        let hwnd = handle_to_hwnd(handle);
        return SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            target.x,
            target.y,
            0,
            0,
            SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        ) != 0;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::move_window(handle, target);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (handle, target);
        false
    }
}

// ──────────────── Window Validation ────────────────

pub fn window_is_valid(handle: i64) -> bool {
    #[cfg(all(windows, has_cpp_backend))]
    unsafe {
        return drag_window_is_valid(handle) != 0;
    }

    #[cfg(all(windows, not(has_cpp_backend)))]
    unsafe {
        use winapi::um::winuser::IsWindow;
        return IsWindow(handle_to_hwnd(handle)) != 0;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::window_is_valid(handle);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = handle;
        false
    }
}

// ──────────────── Cursor Position ────────────────

pub fn current_cursor_position() -> Option<Point> {
    #[cfg(all(windows, has_cpp_backend))]
    unsafe {
        let mut x = 0;
        let mut y = 0;
        if drag_get_cursor_position(&mut x as *mut _, &mut y as *mut _) == 0 {
            return None;
        }
        return Some(Point::new(x, y));
    }

    #[cfg(all(windows, not(has_cpp_backend)))]
    unsafe {
        use winapi::shared::windef::POINT;
        use winapi::um::winuser::GetCursorPos;

        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point as *mut _) == 0 {
            return None;
        }
        return Some(Point::new(point.x, point.y));
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::current_cursor_position();
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

pub fn set_cursor_position(point: Point) -> bool {
    #[cfg(windows)]
    unsafe {
        return winapi::um::winuser::SetCursorPos(point.x, point.y) != 0;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::set_cursor_position(point);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = point;
        false
    }
}

// ──────────────── Mouse Simulation ────────────────

pub fn mouse_left_button_down() -> bool {
    #[cfg(windows)]
    unsafe {
        let mut input: winapi::um::winuser::INPUT = std::mem::zeroed();
        input.type_ = winapi::um::winuser::INPUT_MOUSE;
        *input.u.mi_mut() = winapi::um::winuser::MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: winapi::um::winuser::MOUSEEVENTF_LEFTDOWN,
            time: 0,
            dwExtraInfo: 0,
        };

        return winapi::um::winuser::SendInput(
            1,
            &mut input,
            std::mem::size_of::<winapi::um::winuser::INPUT>() as i32,
        ) == 1;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::mouse_button(1, true);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}

pub fn mouse_left_button_up() -> bool {
    #[cfg(windows)]
    unsafe {
        let mut input: winapi::um::winuser::INPUT = std::mem::zeroed();
        input.type_ = winapi::um::winuser::INPUT_MOUSE;
        *input.u.mi_mut() = winapi::um::winuser::MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: winapi::um::winuser::MOUSEEVENTF_LEFTUP,
            time: 0,
            dwExtraInfo: 0,
        };

        return winapi::um::winuser::SendInput(
            1,
            &mut input,
            std::mem::size_of::<winapi::um::winuser::INPUT>() as i32,
        ) == 1;
    }

    #[cfg(target_os = "linux")]
    {
        return linux_backend::mouse_button(1, false);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}

// ──────────────── Console / UI ────────────────

#[cfg(windows)]
pub fn hide_console_window() {
    unsafe {
        let console = winapi::um::wincon::GetConsoleWindow();
        if !console.is_null() {
            winapi::um::winuser::ShowWindow(console, winapi::um::winuser::SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
pub fn hide_console_window() {}

#[cfg(windows)]
pub fn show_error_dialog(title: &str, message: &str) {
    use std::iter;

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(iter::once(0)).collect()
    }

    let title = wide(title);
    let message = wide(message);

    unsafe {
        winapi::um::winuser::MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            winapi::um::winuser::MB_ICONERROR | winapi::um::winuser::MB_OK,
        );
    }
}

#[cfg(not(windows))]
pub fn show_error_dialog(title: &str, message: &str) {
    eprintln!("[{title}] {message}");
}

// ═══════════════════════════════════════════════════
// Windows Fallback Backend (no C++ compiler)
// ═══════════════════════════════════════════════════

#[cfg(all(windows, not(has_cpp_backend)))]
unsafe fn prepare_foreground_window_fallback(cursor: Point) -> Option<WindowSnapshot> {
    use winapi::shared::windef::{HWND, POINT, RECT};
    use winapi::um::winuser::{
        GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetMonitorInfoW, GetWindowLongW,
        GetWindowRect, IsIconic, IsWindowVisible, IsZoomed, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MonitorFromPoint, MonitorFromWindow, SW_RESTORE, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowPos, ShowWindow, WS_CAPTION, WS_EX_TOPMOST,
    };

    let hwnd = GetForegroundWindow();
    if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
        return None;
    }
    if is_fullscreen_or_unsupported(hwnd) {
        return None;
    }

    let was_maximized = IsZoomed(hwnd) != 0;
    if was_maximized {
        ShowWindow(hwnd, SW_RESTORE);
        std::thread::sleep(Duration::from_millis(10));

        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect as *mut _) == 0 {
            return None;
        }

        let mut monitor_info: MONITORINFO = std::mem::zeroed();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = MonitorFromPoint(
            POINT {
                x: cursor.x,
                y: cursor.y,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        if GetMonitorInfoW(monitor, &mut monitor_info as *mut _) == 0 {
            return None;
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let grip_offset = 24.min((height / 3).max(8));
        let min_x = monitor_info.rcWork.left;
        let max_x = monitor_info.rcWork.right - width;
        let min_y = monitor_info.rcWork.top;
        let max_y = monitor_info.rcWork.bottom - height;
        let target_x = (cursor.x - (width / 2)).clamp(min_x, max_x);
        let target_y = (cursor.y - grip_offset).clamp(min_y, max_y);

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            target_x,
            target_y,
            0,
            0,
            SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut rect as *mut _) == 0 {
        return None;
    }

    Some(WindowSnapshot {
        handle: hwnd as isize as i64,
        position: Point::new(rect.left, rect.top),
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
        was_maximized,
    })
}

#[cfg(all(windows, not(has_cpp_backend)))]
unsafe fn is_fullscreen_or_unsupported(hwnd: winapi::shared::windef::HWND) -> bool {
    use winapi::shared::windef::RECT;
    use winapi::um::winuser::{
        GWL_EXSTYLE, GWL_STYLE, GetMonitorInfoW, GetWindowLongW, GetWindowRect,
        MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, WS_CAPTION, WS_EX_TOPMOST,
    };

    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut rect as *mut _) == 0 {
        return true;
    }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut monitor_info: MONITORINFO = std::mem::zeroed();
    monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut monitor_info as *mut _) == 0 {
        return true;
    }

    let covers_monitor = rect.left <= monitor_info.rcMonitor.left
        && rect.top <= monitor_info.rcMonitor.top
        && rect.right >= monitor_info.rcMonitor.right
        && rect.bottom >= monitor_info.rcMonitor.bottom;
    let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let has_caption = (style & WS_CAPTION as u32) != 0;
    let topmost = (ex_style & WS_EX_TOPMOST as u32) != 0;

    covers_monitor && (!has_caption || topmost)
}

#[cfg(all(windows, not(has_cpp_backend)))]
unsafe fn handle_to_hwnd(handle: i64) -> winapi::shared::windef::HWND {
    handle as isize as winapi::shared::windef::HWND
}
