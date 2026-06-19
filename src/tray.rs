use crate::commands::AppCommand;
use crate::config::{AppConfig, AppPaths};
use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;

#[cfg(windows)]
mod platform {
    #![allow(unsafe_op_in_unsafe_fn)]

    use super::{AppCommand, AppConfig, AppPaths, Result, Sender, anyhow};
    use anyhow::Context;
    use std::ffi::OsStr;
    use std::fs;
    use std::iter;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::thread;
    use winapi::shared::minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM};
    use winapi::shared::windef::{HBRUSH, HICON, HMENU, HWND, POINT};
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::shellapi::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use winapi::um::winuser::{
        AppendMenuW, CREATESTRUCTW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
        DestroyMenu, DispatchMessageW, GWLP_USERDATA, GetCursorPos, GetMessageW, GetWindowLongPtrW,
        IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW, MF_DISABLED, MF_GRAYED,
        MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
        SetForegroundWindow, SetWindowLongPtrW, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
        TrackPopupMenu, TranslateMessage, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY,
        WM_LBUTTONUP, WM_NCCREATE, WM_NCDESTROY, WM_NULL, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
    };

    const WM_TRAYICON: UINT = WM_APP + 1;
    const TRAY_ICON_ID: UINT = 1;
    const MENU_LABEL_APP: usize = 100;
    const MENU_LABEL_STATUS: usize = 101;
    const MENU_ENABLE_DRAGGING: usize = 200;
    const MENU_DISABLE_DRAGGING: usize = 201;
    const MENU_ENABLE_AUTOSTART: usize = 300;
    const MENU_DISABLE_AUTOSTART: usize = 301;
    const MENU_OPEN_SETTINGS: usize = 400;
    const MENU_OPEN_DATA: usize = 401;
    const MENU_OPEN_LOGS: usize = 402;
    const MENU_EXIT: usize = 900;

    static ICON_BYTES: &[u8] = include_bytes!(env!("THREE_WIN_DRAG_ICON_PATH"));

    pub struct TrayHandle {
        hwnd: HWND,
        thread: Option<thread::JoinHandle<()>>,
    }

    struct WindowState {
        menu: HMENU,
        icon: HICON,
        sender: Sender<AppCommand>,
    }

    pub fn build(
        sender: Sender<AppCommand>,
        paths: &AppPaths,
        config: &AppConfig,
    ) -> Result<TrayHandle> {
        paths.ensure_dirs()?;

        let icon_path = paths.data_dir().join("3-win-drag.ico");
        fs::write(&icon_path, ICON_BYTES)
            .with_context(|| format!("failed to write {}", icon_path.display()))?;

        let title = String::from("3-win-drag");
        let status = format!(
            "{} | {} fingers | Sensitivity: {:.2} | Deadzone: {} px",
            config.gesture_action.label(),
            config.gesture_finger_count,
            config.touchpad_sensitivity,
            config.deadzone_pixels
        );

        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = thread::spawn(move || unsafe {
            match create_tray_window(sender, &icon_path, &title, &status) {
                Ok(hwnd) => {
                    let _ = ready_tx.send(Ok(hwnd as isize));
                    message_loop();
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            }
        });

        let hwnd = ready_rx
            .recv()
            .map_err(|_| anyhow!("tray thread exited before initialization"))??;

        Ok(TrayHandle {
            hwnd: hwnd as HWND,
            thread: Some(thread),
        })
    }

    impl Drop for TrayHandle {
        fn drop(&mut self) {
            unsafe {
                if !self.hwnd.is_null() {
                    let _ = PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
                }
            }

            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    unsafe fn create_tray_window(
        sender: Sender<AppCommand>,
        icon_path: &std::path::Path,
        title: &str,
        status: &str,
    ) -> Result<HWND> {
        let instance = GetModuleHandleW(ptr::null());
        if instance.is_null() {
            return Err(anyhow!("failed to get module handle"));
        }

        let class_name = wide("ThreeWinDragTrayWindow");
        let mut wnd_class: WNDCLASSW = mem::zeroed();
        wnd_class.style = 0;
        wnd_class.lpfnWndProc = Some(window_proc);
        wnd_class.hInstance = instance;
        wnd_class.lpszClassName = class_name.as_ptr();
        wnd_class.hbrBackground = ptr::null_mut::<std::ffi::c_void>() as HBRUSH;

        if RegisterClassW(&wnd_class) == 0 {
            return Err(anyhow!("failed to register tray window class"));
        }

        let icon = load_icon_from_file(icon_path)?;
        let menu = build_menu(title, status)?;
        let state = Box::new(WindowState { menu, icon, sender });
        let raw_state = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            raw_state.cast(),
        );

        if hwnd.is_null() {
            let state = Box::from_raw(raw_state);
            let _ = destroy_state(state);
            return Err(anyhow!("failed to create tray window"));
        }

        add_tray_icon(hwnd, instance, title)?;
        update_tray_icon(hwnd, icon)?;

        Ok(hwnd)
    }

    unsafe fn message_loop() {
        let mut message: MSG = mem::zeroed();
        while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe fn build_menu(title: &str, status: &str) -> Result<HMENU> {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return Err(anyhow!("failed to create tray menu"));
        }

        append_menu_string(
            menu,
            MENU_LABEL_APP,
            title,
            MF_STRING | MF_DISABLED | MF_GRAYED,
        )?;
        append_menu_string(
            menu,
            MENU_LABEL_STATUS,
            status,
            MF_STRING | MF_DISABLED | MF_GRAYED,
        )?;
        append_separator(menu)?;
        append_menu_string(menu, MENU_ENABLE_DRAGGING, "Enable dragging", MF_STRING)?;
        append_menu_string(menu, MENU_DISABLE_DRAGGING, "Disable dragging", MF_STRING)?;
        append_separator(menu)?;
        append_menu_string(menu, MENU_ENABLE_AUTOSTART, "Enable auto start", MF_STRING)?;
        append_menu_string(
            menu,
            MENU_DISABLE_AUTOSTART,
            "Disable auto start",
            MF_STRING,
        )?;
        append_separator(menu)?;
        append_menu_string(menu, MENU_OPEN_SETTINGS, "Open settings", MF_STRING)?;
        append_menu_string(menu, MENU_OPEN_DATA, "Open data folder", MF_STRING)?;
        append_menu_string(menu, MENU_OPEN_LOGS, "Open log folder", MF_STRING)?;
        append_separator(menu)?;
        append_menu_string(menu, MENU_EXIT, "Exit", MF_STRING)?;

        Ok(menu)
    }

    unsafe fn append_menu_string(menu: HMENU, id: usize, text: &str, flags: UINT) -> Result<()> {
        let text = wide(text);
        if AppendMenuW(menu, flags, id, text.as_ptr()) == 0 {
            return Err(anyhow!("failed to append tray menu item '{text:?}'"));
        }
        Ok(())
    }

    unsafe fn append_separator(menu: HMENU) -> Result<()> {
        if AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null()) == 0 {
            return Err(anyhow!("failed to append tray separator"));
        }
        Ok(())
    }

    unsafe fn load_icon_from_file(icon_path: &std::path::Path) -> Result<HICON> {
        let wide_path = wide_os(icon_path.as_os_str());
        let icon = LoadImageW(
            ptr::null_mut(),
            wide_path.as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_LOADFROMFILE,
        ) as HICON;

        if icon.is_null() {
            return Err(anyhow!(
                "failed to load tray icon from {}",
                icon_path.display()
            ));
        }

        Ok(icon)
    }

    unsafe fn add_tray_icon(hwnd: HWND, instance: HINSTANCE, title: &str) -> Result<()> {
        let mut tray_data = make_notify_icon_data(hwnd, instance, ptr::null_mut(), title);
        if Shell_NotifyIconW(NIM_ADD, &mut tray_data) == 0 {
            return Err(anyhow!("failed to add tray icon"));
        }
        Ok(())
    }

    unsafe fn update_tray_icon(hwnd: HWND, icon: HICON) -> Result<()> {
        let mut tray_data = make_notify_icon_data(hwnd, ptr::null_mut(), icon, "3-win-drag");
        tray_data.uFlags = NIF_ICON;
        if Shell_NotifyIconW(winapi::um::shellapi::NIM_MODIFY, &mut tray_data) == 0 {
            return Err(anyhow!("failed to update tray icon"));
        }
        Ok(())
    }

    unsafe fn remove_tray_icon(hwnd: HWND) {
        let mut tray_data = make_notify_icon_data(hwnd, ptr::null_mut(), ptr::null_mut(), "");
        let _ = Shell_NotifyIconW(NIM_DELETE, &mut tray_data);
    }

    unsafe fn make_notify_icon_data(
        hwnd: HWND,
        instance: HINSTANCE,
        icon: HICON,
        tooltip: &str,
    ) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = mem::zeroed();
        data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as DWORD;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_ID;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WM_TRAYICON;
        data.hIcon = icon;
        data.hBalloonIcon = ptr::null_mut();
        data.dwState = 0;
        data.dwStateMask = 0;
        data.dwInfoFlags = 0;
        data.guidItem = mem::zeroed();
        data.hWnd = hwnd;
        let tooltip = wide(tooltip);
        let limit = tooltip.len().min(data.szTip.len());
        data.szTip[..limit].copy_from_slice(&tooltip[..limit]);
        let _ = instance;
        data
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam as *const CREATESTRUCTW);
                let state = create.lpCreateParams as isize;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, state);
                1
            }
            WM_COMMAND => {
                if let Some(state) = state_from_hwnd(hwnd) {
                    if let Some(command) = map_command_id((wparam & 0xffff) as usize) {
                        let _ = state.sender.send(command);
                    }
                }
                0
            }
            WM_TRAYICON => {
                let event = lparam as UINT;
                if matches!(event, WM_CONTEXTMENU | WM_RBUTTONUP | WM_LBUTTONUP) {
                    let _ = show_context_menu(hwnd);
                }
                0
            }
            WM_DESTROY => {
                remove_tray_icon(hwnd);
                PostQuitMessage(0);
                0
            }
            WM_NCDESTROY => {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
                if !state_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    let _ = destroy_state(Box::from_raw(state_ptr));
                }
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn show_context_menu(hwnd: HWND) -> Result<()> {
        let Some(state) = state_from_hwnd(hwnd) else {
            return Err(anyhow!("tray state was not available"));
        };

        let mut cursor: POINT = mem::zeroed();
        if GetCursorPos(&mut cursor) == 0 {
            return Err(anyhow!("failed to query cursor position"));
        }

        let _ = SetForegroundWindow(hwnd);
        if TrackPopupMenu(
            state.menu,
            TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            0,
            hwnd,
            ptr::null(),
        ) == 0
        {
            return Err(anyhow!("failed to display tray menu"));
        }

        let _ = PostMessageW(hwnd, WM_NULL, 0, 0);
        Ok(())
    }

    unsafe fn destroy_state(state: Box<WindowState>) -> Result<()> {
        if !state.menu.is_null() {
            let _ = DestroyMenu(state.menu);
        }
        if !state.icon.is_null() {
            let _ = DestroyIcon(state.icon);
        }
        Ok(())
    }

    unsafe fn state_from_hwnd(hwnd: HWND) -> Option<&'static WindowState> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
        if ptr.is_null() { None } else { Some(&*ptr) }
    }

    fn map_command_id(id: usize) -> Option<AppCommand> {
        match id {
            MENU_ENABLE_DRAGGING => Some(AppCommand::EnableDragging),
            MENU_DISABLE_DRAGGING => Some(AppCommand::DisableDragging),
            MENU_ENABLE_AUTOSTART => Some(AppCommand::EnableAutoStart),
            MENU_DISABLE_AUTOSTART => Some(AppCommand::DisableAutoStart),
            MENU_OPEN_SETTINGS => Some(AppCommand::OpenSettings),
            MENU_OPEN_DATA => Some(AppCommand::OpenDataDirectory),
            MENU_OPEN_LOGS => Some(AppCommand::OpenLogDirectory),
            MENU_EXIT => Some(AppCommand::Exit),
            _ => None,
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }

    fn wide_os(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{AppCommand, AppConfig, AppPaths, Result, Sender, anyhow};
    use tray_item::TrayItem;
    use std::sync::Arc;

    pub struct TrayHandle {
        _tray: TrayItem,
    }

    pub fn build(
        sender: Sender<AppCommand>,
        _paths: &AppPaths,
        config: &AppConfig,
    ) -> Result<TrayHandle> {
        let mut tray = TrayItem::new("3-win-drag", "APPICON")
            .map_err(|error| anyhow!("failed to create tray icon: {error}"))?;

        tray.add_label("3-win-drag")
            .map_err(|error| anyhow!("failed to add tray label: {error}"))?;
        tray.add_label(&format!(
            "{} | {} fingers | Sensitivity: {:.2} | Deadzone: {} px",
            config.gesture_action.label(),
            config.gesture_finger_count,
            config.touchpad_sensitivity,
            config.deadzone_pixels
        ))
        .map_err(|error| anyhow!("failed to add tray label: {error}"))?;

        add_action(
            &mut tray,
            &sender,
            "Enable dragging",
            AppCommand::EnableDragging,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Disable dragging",
            AppCommand::DisableDragging,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Enable auto start",
            AppCommand::EnableAutoStart,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Disable auto start",
            AppCommand::DisableAutoStart,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Open settings",
            AppCommand::OpenSettings,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Open data folder",
            AppCommand::OpenDataDirectory,
        )?;
        add_action(
            &mut tray,
            &sender,
            "Open log folder",
            AppCommand::OpenLogDirectory,
        )?;
        add_action(&mut tray, &sender, "Exit", AppCommand::Exit)?;

        Ok(TrayHandle { _tray: tray })
    }

    fn add_action(
        tray: &mut TrayItem,
        sender: &Sender<AppCommand>,
        label: &str,
        command: AppCommand,
    ) -> Result<()> {
        let sender = sender.clone();
        let command = Arc::new(command);
        tray.add_menu_item(label, move || {
            let _ = sender.send((*command).clone());
        })
        .map_err(|error| anyhow!("failed to add tray menu item '{label}': {error}"))
    }
}

pub use platform::build;
