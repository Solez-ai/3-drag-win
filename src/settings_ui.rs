use crate::commands::AppCommand;
use crate::config::{AppConfig, AppPaths, GestureAction};
use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub manufacturer: String,
    pub model: String,
    pub touchpads: Vec<String>,
    pub recommended_template: String,
    pub recommendation_reason: String,
}

#[derive(Debug, Clone)]
pub struct TemplateProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub gesture_action: GestureAction,
    pub gesture_finger_count: u8,
    pub touchpad_sensitivity: f32,
    pub deadzone_pixels: i32,
    pub minimum_update_interval_ms: u64,
    pub smoothing_factor: f32,
    pub ignore_fullscreen_windows: bool,
}

pub fn templates() -> Vec<TemplateProfile> {
    vec![
        TemplateProfile {
            id: "balanced",
            name: "Balanced Precision",
            description: "General-purpose profile for most Windows Precision Touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.78,
            deadzone_pixels: 7,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.82,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "drag_drop_precise",
            name: "Drag And Drop",
            description: "Optimized for browser tabs, files, downloads, and image drag and drop.",
            gesture_action: GestureAction::MouseDrag,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.68,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.8,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "lenovo_precision",
            name: "Lenovo Precision",
            description: "Slightly calmer motion tuned for common Lenovo Precision Touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.72,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.84,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "dell_precision",
            name: "Dell Precision",
            description: "A little faster with moderate smoothing for Dell laptops.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.8,
            deadzone_pixels: 7,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.8,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "hp_precision",
            name: "HP Precision",
            description: "Conservative jitter control for HP and Pavilion style touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.7,
            deadzone_pixels: 9,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.85,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "asus_precision",
            name: "ASUS Precision",
            description: "Balanced response for ASUS and ROG touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.76,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.83,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "surface_precision",
            name: "Surface Precision",
            description: "Higher fidelity tuning for Microsoft Surface devices.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.74,
            deadzone_pixels: 6,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.86,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "synaptics_safe",
            name: "Synaptics Safe",
            description: "Very stable settings for noisier or legacy HID touchpad reports.",
            gesture_action: GestureAction::MouseDrag,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.58,
            deadzone_pixels: 10,
            minimum_update_interval_ms: 6,
            smoothing_factor: 0.74,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "elan_balanced",
            name: "ELAN Balanced",
            description: "Extra filtering for ELAN and similar touchpads that report noisier deltas.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.64,
            deadzone_pixels: 10,
            minimum_update_interval_ms: 6,
            smoothing_factor: 0.78,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "acer_precision",
            name: "Acer Precision",
            description: "Moderately damped tuning for Acer Swift, Aspire, and Spin class touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.73,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.84,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "framework_precision",
            name: "Framework Precision",
            description: "Slightly slower but very stable profile for Framework laptops and high-resolution touchpads.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.71,
            deadzone_pixels: 7,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.86,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "msi_precision",
            name: "MSI Precision",
            description: "Faster response for MSI laptops while preserving enough smoothing for desktop dragging.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.79,
            deadzone_pixels: 7,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.8,
            ignore_fullscreen_windows: true,
        },
        TemplateProfile {
            id: "samsung_precision",
            name: "Samsung Precision",
            description: "Stable vendor profile for Samsung Galaxy Book devices.",
            gesture_action: GestureAction::WindowMove,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.72,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 5,
            smoothing_factor: 0.84,
            ignore_fullscreen_windows: true,
        },
    ]
}

pub fn apply_template(template_id: &str, current: &AppConfig) -> Option<AppConfig> {
    let template = templates()
        .into_iter()
        .find(|template| template.id == template_id)?;

    Some(AppConfig {
        enabled: current.enabled,
        launch_at_startup: current.launch_at_startup,
        touchpad_profile: template.id.to_string(),
        gesture_action: template.gesture_action,
        gesture_finger_count: template.gesture_finger_count,
        touchpad_sensitivity: template.touchpad_sensitivity,
        deadzone_pixels: template.deadzone_pixels,
        minimum_update_interval_ms: template.minimum_update_interval_ms,
        smoothing_factor: template.smoothing_factor,
        ignore_fullscreen_windows: template.ignore_fullscreen_windows,
    })
}

pub fn detect_hardware() -> HardwareInfo {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "$c=Get-CimInstance Win32_ComputerSystem | Select-Object Manufacturer,Model; \
             $t=@(Get-PnpDevice | Where-Object { $_.FriendlyName -match 'touch ?pad|precision touchpad' -or $_.InstanceId -match 'PNP0C50' } | Select-Object -ExpandProperty FriendlyName); \
             [PSCustomObject]@{manufacturer=$c.Manufacturer; model=$c.Model; touchpads=$t} | ConvertTo-Json -Compress",
        ])
        .output();

    let mut info = match output {
        Ok(output) if output.status.success() => parse_hardware_output(&output.stdout),
        _ => HardwareInfo {
            manufacturer: String::from("Unknown"),
            model: String::from("Unknown"),
            touchpads: Vec::new(),
            recommended_template: String::from("balanced"),
            recommendation_reason: String::from(
                "Hardware detection was unavailable. Falling back to the balanced profile.",
            ),
        },
    };

    let (recommended_template, reason) = recommend_template(&info);
    info.recommended_template = recommended_template.to_string();
    info.recommendation_reason = reason;
    info
}

fn parse_hardware_output(output: &[u8]) -> HardwareInfo {
    let value: serde_json::Value = serde_json::from_slice(output).unwrap_or_default();
    let manufacturer = value
        .get("manufacturer")
        .and_then(|value| value.as_str())
        .unwrap_or("Unknown")
        .trim()
        .to_string();
    let model = value
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("Unknown")
        .trim()
        .to_string();
    let touchpads = value
        .get("touchpads")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    HardwareInfo {
        manufacturer,
        model,
        touchpads,
        recommended_template: String::new(),
        recommendation_reason: String::new(),
    }
}

fn recommend_template(info: &HardwareInfo) -> (&'static str, String) {
    let manufacturer = info.manufacturer.to_ascii_lowercase();
    let model = info.model.to_ascii_lowercase();
    let touchpads = info.touchpads.join(" ").to_ascii_lowercase();

    if touchpads.contains("synaptics") {
        return (
            "synaptics_safe",
            String::from(
                "Detected a Synaptics touchpad signature, so the safest low-jitter profile was selected.",
            ),
        );
    }
    if touchpads.contains("elan") {
        return (
            "elan_balanced",
            String::from(
                "Detected an ELAN touchpad signature, so a more heavily filtered balanced profile was selected.",
            ),
        );
    }
    if manufacturer.contains("lenovo") {
        return (
            "lenovo_precision",
            format!(
                "Detected manufacturer '{}' and matched the Lenovo Precision template.",
                info.manufacturer
            ),
        );
    }
    if manufacturer.contains("acer") {
        return (
            "acer_precision",
            format!(
                "Detected manufacturer '{}' and matched the Acer Precision template.",
                info.manufacturer
            ),
        );
    }
    if manufacturer.contains("dell") {
        return (
            "dell_precision",
            format!(
                "Detected manufacturer '{}' and matched the Dell Precision template.",
                info.manufacturer
            ),
        );
    }
    if manufacturer.contains("framework") || model.contains("framework") {
        return (
            "framework_precision",
            format!(
                "Detected '{}' / '{}' and matched the Framework Precision template.",
                info.manufacturer, info.model
            ),
        );
    }
    if manufacturer.contains("hewlett") || manufacturer == "hp" || manufacturer.contains("hp") {
        return (
            "hp_precision",
            format!(
                "Detected manufacturer '{}' and matched the HP Precision template.",
                info.manufacturer
            ),
        );
    }
    if manufacturer.contains("micro-star") || manufacturer.contains("msi") || model.contains("msi")
    {
        return (
            "msi_precision",
            format!(
                "Detected '{}' / '{}' and matched the MSI Precision template.",
                info.manufacturer, info.model
            ),
        );
    }
    if manufacturer.contains("asus") || model.contains("rog") {
        return (
            "asus_precision",
            format!(
                "Detected '{}' / '{}' and matched the ASUS Precision template.",
                info.manufacturer, info.model
            ),
        );
    }
    if manufacturer.contains("samsung") {
        return (
            "samsung_precision",
            format!(
                "Detected manufacturer '{}' and matched the Samsung Precision template.",
                info.manufacturer
            ),
        );
    }
    if manufacturer.contains("microsoft") || model.contains("surface") {
        return (
            "surface_precision",
            format!(
                "Detected '{}' / '{}' and matched the Surface Precision template.",
                info.manufacturer, info.model
            ),
        );
    }

    (
        "balanced",
        String::from("No vendor-specific match was found, so the balanced profile was selected."),
    )
}

#[cfg(windows)]
mod platform {
    #![allow(unsafe_op_in_unsafe_fn)]

    use super::{
        AppCommand, AppConfig, AppPaths, GestureAction, HardwareInfo, Result, Sender,
        TemplateProfile, anyhow, apply_template, detect_hardware, templates,
    };
    use anyhow::Context;
    use std::ffi::OsStr;
    use std::fs;
    use std::iter;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::thread;
    use winapi::shared::basetsd::LONG_PTR;
    use winapi::shared::minwindef::{HIWORD, LOWORD, LPARAM, LRESULT, UINT, WPARAM};
    use winapi::shared::windef::{HBRUSH, HICON, HMENU, HWND};
    use winapi::shared::winerror::ERROR_CLASS_ALREADY_EXISTS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{
        BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BST_CHECKED, BST_UNCHECKED, CB_ADDSTRING,
        CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST,
        CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
        DispatchMessageW, ES_AUTOHSCROLL, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW,
        GetWindowTextLengthW, GetWindowTextW, ICON_BIG, ICON_SMALL, IMAGE_ICON, IsWindowVisible,
        LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW, MSG, PostMessageW, PostQuitMessage,
        RegisterClassW, SW_HIDE, SW_SHOW, SendMessageW, SetFocus, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage, UpdateWindow, WM_APP,
        WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE, WNDCLASSW, WS_BORDER, WS_CHILD,
        WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
    };

    const WM_SETTINGS_OPEN: UINT = WM_APP + 50;
    const WM_SETTINGS_REFRESH: UINT = WM_APP + 51;
    const WM_SETTINGS_SHUTDOWN: UINT = WM_APP + 52;

    const ID_PROFILE: i32 = 200;
    const ID_ACTION: i32 = 202;
    const ID_FINGER_COUNT: i32 = 203;
    const ID_SENSITIVITY: i32 = 204;
    const ID_DEADZONE: i32 = 205;
    const ID_MIN_INTERVAL: i32 = 206;
    const ID_SMOOTHING: i32 = 207;
    const ID_ENABLED: i32 = 208;
    const ID_AUTOSTART: i32 = 209;
    const ID_FULLSCREEN: i32 = 210;
    const ID_APPLY: i32 = 300;
    const ID_RECOMMENDED_BUTTON: i32 = 301;
    const ID_RELOAD: i32 = 302;

    static ICON_BYTES: &[u8] = include_bytes!(env!("THREE_WIN_DRAG_ICON_PATH"));

    #[derive(Default)]
    struct ControlHandles {
        hardware: HWND,
        recommended: HWND,
        profile: HWND,
        template_desc: HWND,
        action: HWND,
        finger_count: HWND,
        sensitivity: HWND,
        deadzone: HWND,
        min_interval: HWND,
        smoothing: HWND,
        enabled: HWND,
        autostart: HWND,
        fullscreen: HWND,
        status: HWND,
        apply: HWND,
    }

    struct WindowState {
        paths: AppPaths,
        sender: Sender<AppCommand>,
        hardware: HardwareInfo,
        templates: Vec<TemplateProfile>,
        controls: ControlHandles,
        icon: HICON,
        suspend_events: bool,
    }

    pub struct SettingsWindowHandle {
        hwnd: HWND,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl SettingsWindowHandle {
        pub fn open(&self) -> Result<()> {
            unsafe {
                if PostMessageW(self.hwnd, WM_SETTINGS_OPEN, 0, 0) == 0 {
                    return Err(anyhow!("failed to open settings window"));
                }
            }
            Ok(())
        }

        pub fn refresh(&self) {
            unsafe {
                let _ = PostMessageW(self.hwnd, WM_SETTINGS_REFRESH, 0, 0);
            }
        }
    }

    impl Drop for SettingsWindowHandle {
        fn drop(&mut self) {
            unsafe {
                if !self.hwnd.is_null() {
                    let _ = PostMessageW(self.hwnd, WM_SETTINGS_SHUTDOWN, 0, 0);
                }
            }

            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn spawn_window(
        paths: AppPaths,
        sender: Sender<AppCommand>,
    ) -> Result<SettingsWindowHandle> {
        let hardware = detect_hardware();
        let template_profiles = templates();
        let icon_path = ensure_icon_file(&paths)?;

        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = thread::spawn(move || unsafe {
            match create_window(paths, sender, hardware, template_profiles, &icon_path) {
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
            .map_err(|_| anyhow!("settings window thread exited before initialization"))??;

        Ok(SettingsWindowHandle {
            hwnd: hwnd as HWND,
            thread: Some(thread),
        })
    }

    unsafe fn create_window(
        paths: AppPaths,
        sender: Sender<AppCommand>,
        hardware: HardwareInfo,
        templates: Vec<TemplateProfile>,
        icon_path: &std::path::Path,
    ) -> Result<HWND> {
        let instance = GetModuleHandleW(ptr::null());
        if instance.is_null() {
            return Err(anyhow!("failed to get module handle"));
        }

        let class_name = wide("ThreeWinDragSettingsWindow");
        let mut wnd_class: WNDCLASSW = mem::zeroed();
        wnd_class.style = 0;
        wnd_class.lpfnWndProc = Some(window_proc);
        wnd_class.hInstance = instance;
        wnd_class.lpszClassName = class_name.as_ptr();
        wnd_class.hbrBackground = 16 as HBRUSH;

        if RegisterClassW(&wnd_class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            return Err(anyhow!("failed to register settings window class"));
        }

        let icon = load_icon_from_file(icon_path).unwrap_or(ptr::null_mut());
        let state = Box::new(WindowState {
            paths,
            sender,
            hardware,
            templates,
            controls: ControlHandles::default(),
            icon,
            suspend_events: false,
        });
        let raw_state = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name.as_ptr(),
            wide("3-win-drag Settings").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            180,
            120,
            780,
            640,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            raw_state.cast(),
        );

        if hwnd.is_null() {
            let mut state = Box::from_raw(raw_state);
            if !state.icon.is_null() {
                let _ = DestroyIcon(state.icon);
                state.icon = ptr::null_mut();
            }
            return Err(anyhow!("failed to create settings window"));
        }

        Ok(hwnd)
    }

    unsafe fn message_loop() {
        let mut message: MSG = mem::zeroed();
        while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
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
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as LONG_PTR);
                return 1;
            }
            WM_CREATE => {
                let Some(state) = window_state_mut(hwnd) else {
                    return -1;
                };

                create_controls(hwnd, state);
                if !state.icon.is_null() {
                    SendMessageW(hwnd, 0x0080, ICON_BIG as usize, state.icon as LPARAM);
                    SendMessageW(hwnd, 0x0080, ICON_SMALL as usize, state.icon as LPARAM);
                }
                load_form_from_disk(state);
                set_status(state, "Settings are ready.");
                return 0;
            }
            WM_COMMAND => {
                let Some(state) = window_state_mut(hwnd) else {
                    return 0;
                };

                let control_id = LOWORD(wparam as u32) as i32;
                let notify_code = HIWORD(wparam as u32) as u16;

                match (control_id, notify_code) {
                    (ID_APPLY, BN_CLICKED) => {
                        apply_form(hwnd, state);
                        return 0;
                    }
                    (ID_RECOMMENDED_BUTTON, BN_CLICKED) => {
                        apply_recommended_template(hwnd, state);
                        return 0;
                    }
                    (ID_RELOAD, BN_CLICKED) => {
                        load_form_from_disk(state);
                        set_status(state, "Reloaded settings from disk.");
                        return 0;
                    }
                    (ID_PROFILE, CBN_SELCHANGE) if !state.suspend_events => {
                        preview_selected_template(state);
                        return 0;
                    }
                    _ => {}
                }
            }
            WM_SETTINGS_OPEN => {
                if let Some(state) = window_state_mut(hwnd) {
                    load_form_from_disk(state);
                    ShowWindow(hwnd, SW_SHOW);
                    UpdateWindow(hwnd);
                    SetForegroundWindow(hwnd);
                    SetFocus(state.controls.apply);
                }
                return 0;
            }
            WM_SETTINGS_REFRESH => {
                if let Some(state) = window_state_mut(hwnd) {
                    if IsWindowVisible(hwnd) != 0 {
                        load_form_from_disk(state);
                    }
                }
                return 0;
            }
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                return 0;
            }
            WM_SETTINGS_SHUTDOWN => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_DESTROY => {
                if let Some(raw) = take_window_state(hwnd) {
                    destroy_state(raw);
                }
                PostQuitMessage(0);
                return 0;
            }
            _ => {}
        }

        DefWindowProcW(hwnd, message, wparam, lparam)
    }

    unsafe fn create_controls(hwnd: HWND, state: &mut WindowState) {
        create_label(hwnd, "Detected hardware", 20, 18, 160, 20);
        state.controls.hardware = create_label(hwnd, "", 20, 40, 720, 36);

        create_label(hwnd, "Recommended template", 20, 86, 180, 20);
        state.controls.recommended = create_label(hwnd, "", 20, 108, 720, 40);

        create_label(hwnd, "Touchpad profile", 20, 164, 160, 20);
        state.controls.profile = create_combo(hwnd, ID_PROFILE, 20, 186, 320, 220);

        create_label(hwnd, "Template description", 370, 164, 160, 20);
        state.controls.template_desc = create_label(hwnd, "", 370, 186, 370, 54);

        create_label(hwnd, "Gesture action", 20, 252, 140, 20);
        state.controls.action = create_combo(hwnd, ID_ACTION, 20, 274, 220, 120);
        add_combo_item(state.controls.action, "Window move");
        add_combo_item(state.controls.action, "Mouse drag");

        create_label(hwnd, "Finger count", 270, 252, 120, 20);
        state.controls.finger_count = create_input(hwnd, ID_FINGER_COUNT, 270, 274, 110);

        create_label(hwnd, "Sensitivity", 400, 252, 120, 20);
        state.controls.sensitivity = create_input(hwnd, ID_SENSITIVITY, 400, 274, 110);

        create_label(hwnd, "Deadzone pixels", 530, 252, 120, 20);
        state.controls.deadzone = create_input(hwnd, ID_DEADZONE, 530, 274, 110);

        create_label(hwnd, "Minimum update interval (ms)", 20, 326, 220, 20);
        state.controls.min_interval = create_input(hwnd, ID_MIN_INTERVAL, 20, 348, 160);

        create_label(hwnd, "Smoothing factor", 210, 326, 160, 20);
        state.controls.smoothing = create_input(hwnd, ID_SMOOTHING, 210, 348, 160);

        state.controls.enabled =
            create_checkbox(hwnd, ID_ENABLED, "Dragging enabled", 20, 402, 180, 22);
        state.controls.autostart =
            create_checkbox(hwnd, ID_AUTOSTART, "Launch at startup", 220, 402, 180, 22);
        state.controls.fullscreen = create_checkbox(
            hwnd,
            ID_FULLSCREEN,
            "Ignore full-screen windows",
            420,
            402,
            240,
            22,
        );

        state.controls.apply = create_button(hwnd, ID_APPLY, "Save And Apply", 20, 456, 150, 34);
        create_button(
            hwnd,
            ID_RECOMMENDED_BUTTON,
            "Apply Recommended",
            184,
            456,
            170,
            34,
        );
        create_button(hwnd, ID_RELOAD, "Reload", 368, 456, 120, 34);

        state.controls.status = create_label(hwnd, "", 20, 510, 720, 32);

        populate_profile_combo(state);
    }

    unsafe fn populate_profile_combo(state: &WindowState) {
        SendMessageW(state.controls.profile, CB_RESETCONTENT, 0, 0);
        for template in &state.templates {
            add_combo_item(state.controls.profile, template.name);
        }
    }

    unsafe fn load_form_from_disk(state: &mut WindowState) {
        let config = AppConfig::load_or_create(&state.paths).unwrap_or_default();
        load_form_from_config(state, &config);
    }

    unsafe fn load_form_from_config(state: &mut WindowState, config: &AppConfig) {
        state.suspend_events = true;
        set_text(
            state.controls.hardware,
            &format!(
                "{} {}{}",
                state.hardware.manufacturer,
                state.hardware.model,
                format_touchpads(&state.hardware.touchpads)
            ),
        );
        set_text(
            state.controls.recommended,
            &format!(
                "{}. {}",
                template_name(&state.templates, &state.hardware.recommended_template),
                state.hardware.recommendation_reason
            ),
        );
        set_checkbox(state.controls.enabled, config.enabled);
        set_checkbox(state.controls.autostart, config.launch_at_startup);
        set_checkbox(state.controls.fullscreen, config.ignore_fullscreen_windows);
        set_combo_selection(
            state.controls.profile,
            template_index(&state.templates, &config.touchpad_profile).unwrap_or(0),
        );
        set_combo_selection(
            state.controls.action,
            action_to_index(config.gesture_action),
        );
        set_text(
            state.controls.finger_count,
            &config.gesture_finger_count.to_string(),
        );
        set_text(
            state.controls.sensitivity,
            &format!("{:.2}", config.touchpad_sensitivity),
        );
        set_text(state.controls.deadzone, &config.deadzone_pixels.to_string());
        set_text(
            state.controls.min_interval,
            &config.minimum_update_interval_ms.to_string(),
        );
        set_text(
            state.controls.smoothing,
            &format!("{:.2}", config.smoothing_factor),
        );
        update_template_description(state);
        state.suspend_events = false;
    }

    unsafe fn preview_selected_template(state: &mut WindowState) {
        let Some(template) = selected_template(state) else {
            return;
        };

        let preview = AppConfig {
            enabled: checkbox_value(state.controls.enabled),
            launch_at_startup: checkbox_value(state.controls.autostart),
            touchpad_profile: template.id.to_string(),
            gesture_action: template.gesture_action,
            gesture_finger_count: template.gesture_finger_count,
            touchpad_sensitivity: template.touchpad_sensitivity,
            deadzone_pixels: template.deadzone_pixels,
            minimum_update_interval_ms: template.minimum_update_interval_ms,
            smoothing_factor: template.smoothing_factor,
            ignore_fullscreen_windows: template.ignore_fullscreen_windows,
        };

        load_form_from_config(state, &preview);
        set_status(
            state,
            "Loaded the selected template into the form. Click Save And Apply to commit it.",
        );
    }

    unsafe fn apply_form(hwnd: HWND, state: &mut WindowState) {
        match collect_form_config(state).and_then(|config| persist_and_send_config(state, &config))
        {
            Ok(()) => {
                load_form_from_disk(state);
                set_status(state, "Settings applied.");
                SetForegroundWindow(hwnd);
            }
            Err(error) => {
                set_status(state, &format!("Apply failed: {error}"));
            }
        }
    }

    unsafe fn apply_recommended_template(hwnd: HWND, state: &mut WindowState) {
        let current = match collect_form_config(state) {
            Ok(config) => config,
            Err(error) => {
                set_status(state, &format!("Cannot apply template: {error}"));
                return;
            }
        };

        let Some(config) = apply_template(&state.hardware.recommended_template, &current) else {
            set_status(state, "The recommended template is unavailable.");
            return;
        };

        match persist_and_send_config(state, &config) {
            Ok(()) => {
                load_form_from_config(state, &config);
                set_status(
                    state,
                    &format!(
                        "Applied recommended template: {}.",
                        template_name(&state.templates, &config.touchpad_profile)
                    ),
                );
                SetForegroundWindow(hwnd);
            }
            Err(error) => set_status(state, &format!("Apply failed: {error}")),
        }
    }

    unsafe fn persist_and_send_config(state: &WindowState, config: &AppConfig) -> Result<()> {
        config.save(&state.paths)?;
        state
            .sender
            .send(AppCommand::ApplyConfig(config.clone()))
            .map_err(|_| anyhow!("failed to send settings update to the running app"))
    }

    unsafe fn collect_form_config(state: &WindowState) -> Result<AppConfig> {
        let touchpad_profile = selected_template(state)
            .map(|template| template.id.to_string())
            .ok_or_else(|| anyhow!("no touchpad profile is selected"))?;

        Ok(AppConfig {
            enabled: checkbox_value(state.controls.enabled),
            launch_at_startup: checkbox_value(state.controls.autostart),
            touchpad_profile,
            gesture_action: index_to_action(combo_selection(state.controls.action)),
            gesture_finger_count: parse_u8(state.controls.finger_count, 3, 5)?,
            touchpad_sensitivity: parse_f32(state.controls.sensitivity, 0.20, 2.0)?,
            deadzone_pixels: parse_i32(state.controls.deadzone, 1, 30)?,
            minimum_update_interval_ms: parse_u64(state.controls.min_interval, 1, 20)?,
            smoothing_factor: parse_f32(state.controls.smoothing, 0.10, 1.0)?,
            ignore_fullscreen_windows: checkbox_value(state.controls.fullscreen),
        })
    }

    unsafe fn update_template_description(state: &WindowState) {
        if let Some(template) = selected_template(state) {
            set_text(
                state.controls.template_desc,
                &format!(
                    "{} Action: {} | Sensitivity: {:.2} | Deadzone: {} | Smoothing: {:.2}",
                    template.description,
                    template.gesture_action.label(),
                    template.touchpad_sensitivity,
                    template.deadzone_pixels,
                    template.smoothing_factor
                ),
            );
        }
    }

    unsafe fn create_label(hwnd: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            w,
            h,
            hwnd,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }

    unsafe fn create_input(hwnd: HWND, id: i32, x: i32, y: i32, w: i32) -> HWND {
        CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL,
            x,
            y,
            w,
            24,
            hwnd,
            id_to_menu(id),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }

    unsafe fn create_combo(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
        CreateWindowExW(
            0,
            wide("COMBOBOX").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | CBS_DROPDOWNLIST,
            x,
            y,
            w,
            h,
            hwnd,
            id_to_menu(id),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }

    unsafe fn create_checkbox(
        hwnd: HWND,
        id: i32,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x00000003,
            x,
            y,
            w,
            h,
            hwnd,
            id_to_menu(id),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }

    unsafe fn create_button(
        hwnd: HWND,
        id: i32,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            x,
            y,
            w,
            h,
            hwnd,
            id_to_menu(id),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }

    unsafe fn add_combo_item(combo: HWND, text: &str) {
        SendMessageW(combo, CB_ADDSTRING, 0, wide(text).as_ptr() as LPARAM);
    }

    unsafe fn set_combo_selection(combo: HWND, index: usize) {
        SendMessageW(combo, CB_SETCURSEL, index, 0);
    }

    unsafe fn combo_selection(combo: HWND) -> usize {
        let index = SendMessageW(combo, CB_GETCURSEL, 0, 0);
        if index < 0 { 0 } else { index as usize }
    }

    unsafe fn set_text(hwnd: HWND, text: &str) {
        SetWindowTextW(hwnd, wide(text).as_ptr());
    }

    unsafe fn text(hwnd: HWND) -> String {
        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return String::new();
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..copied as usize])
    }

    unsafe fn set_checkbox(hwnd: HWND, value: bool) {
        SendMessageW(
            hwnd,
            BM_SETCHECK,
            if value {
                BST_CHECKED as usize
            } else {
                BST_UNCHECKED as usize
            },
            0,
        );
    }

    unsafe fn checkbox_value(hwnd: HWND) -> bool {
        SendMessageW(hwnd, BM_GETCHECK, 0, 0) as usize == BST_CHECKED
    }

    unsafe fn selected_template<'a>(state: &'a WindowState) -> Option<&'a TemplateProfile> {
        state.templates.get(combo_selection(state.controls.profile))
    }

    unsafe fn parse_u8(hwnd: HWND, min: u8, max: u8) -> Result<u8> {
        let raw = text(hwnd);
        let parsed = raw
            .trim()
            .parse::<u8>()
            .map_err(|_| anyhow!("expected a whole number between {min} and {max}"))?;
        Ok(parsed.clamp(min, max))
    }

    unsafe fn parse_i32(hwnd: HWND, min: i32, max: i32) -> Result<i32> {
        let raw = text(hwnd);
        let parsed = raw
            .trim()
            .parse::<i32>()
            .map_err(|_| anyhow!("expected a whole number between {min} and {max}"))?;
        Ok(parsed.clamp(min, max))
    }

    unsafe fn parse_u64(hwnd: HWND, min: u64, max: u64) -> Result<u64> {
        let raw = text(hwnd);
        let parsed = raw
            .trim()
            .parse::<u64>()
            .map_err(|_| anyhow!("expected a whole number between {min} and {max}"))?;
        Ok(parsed.clamp(min, max))
    }

    unsafe fn parse_f32(hwnd: HWND, min: f32, max: f32) -> Result<f32> {
        let raw = text(hwnd);
        let parsed = raw
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow!("expected a number between {min:.2} and {max:.2}"))?;
        Ok(parsed.clamp(min, max))
    }

    fn template_index(templates: &[TemplateProfile], template_id: &str) -> Option<usize> {
        templates
            .iter()
            .position(|template| template.id == template_id)
    }

    fn template_name(templates: &[TemplateProfile], template_id: &str) -> &'static str {
        templates
            .iter()
            .find(|template| template.id == template_id)
            .map(|template| template.name)
            .unwrap_or("Balanced Precision")
    }

    fn format_touchpads(touchpads: &[String]) -> String {
        if touchpads.is_empty() {
            String::new()
        } else {
            format!(" | Touchpad: {}", touchpads.join(", "))
        }
    }

    fn action_to_index(action: GestureAction) -> usize {
        match action {
            GestureAction::WindowMove => 0,
            GestureAction::MouseDrag => 1,
        }
    }

    fn index_to_action(index: usize) -> GestureAction {
        match index {
            1 => GestureAction::MouseDrag,
            _ => GestureAction::WindowMove,
        }
    }

    unsafe fn set_status(state: &WindowState, message: &str) {
        set_text(state.controls.status, message);
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
                "failed to load settings icon from {}",
                icon_path.display()
            ));
        }

        Ok(icon)
    }

    fn ensure_icon_file(paths: &AppPaths) -> Result<std::path::PathBuf> {
        paths.ensure_dirs()?;
        let icon_path = paths.data_dir().join("3-win-drag.ico");
        if !icon_path.exists() {
            fs::write(&icon_path, ICON_BYTES)
                .with_context(|| format!("failed to write {}", icon_path.display()))?;
        }
        Ok(icon_path)
    }

    unsafe fn window_state_mut(hwnd: HWND) -> Option<&'static mut WindowState> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
        ptr.as_mut()
    }

    unsafe fn take_window_state(hwnd: HWND) -> Option<Box<WindowState>> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
        if ptr.is_null() {
            None
        } else {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            Some(Box::from_raw(ptr))
        }
    }

    unsafe fn destroy_state(mut state: Box<WindowState>) {
        if !state.icon.is_null() {
            let _ = DestroyIcon(state.icon);
            state.icon = ptr::null_mut();
        }
    }

    fn id_to_menu(id: i32) -> HMENU {
        id as isize as HMENU
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(iter::once(0)).collect()
    }

    fn wide_os(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{AppPaths, Result, Sender};
    use crate::commands::AppCommand;

    pub struct SettingsWindowHandle;

    impl SettingsWindowHandle {
        pub fn open(&self) -> Result<()> {
            Ok(())
        }

        pub fn refresh(&self) {}
    }

    pub fn spawn_window(
        _paths: AppPaths,
        _sender: Sender<AppCommand>,
    ) -> Result<SettingsWindowHandle> {
        Ok(SettingsWindowHandle)
    }
}

pub use platform::{SettingsWindowHandle, spawn_window};

#[cfg(test)]
mod tests {
    use super::{HardwareInfo, recommend_template};

    fn hardware(manufacturer: &str, model: &str, touchpads: &[&str]) -> HardwareInfo {
        HardwareInfo {
            manufacturer: manufacturer.to_string(),
            model: model.to_string(),
            touchpads: touchpads.iter().map(|value| value.to_string()).collect(),
            recommended_template: String::new(),
            recommendation_reason: String::new(),
        }
    }

    #[test]
    fn recommends_synaptics_template_from_touchpad_name() {
        let (template, _) = recommend_template(&hardware(
            "Generic",
            "Laptop",
            &["Synaptics Precision Touchpad"],
        ));
        assert_eq!(template, "synaptics_safe");
    }

    #[test]
    fn recommends_vendor_template_from_manufacturer() {
        let (template, _) = recommend_template(&hardware("Lenovo", "ThinkPad", &[]));
        assert_eq!(template, "lenovo_precision");
    }

    #[test]
    fn recommends_framework_template_from_model() {
        let (template, _) = recommend_template(&hardware("Framework", "Laptop 13", &[]));
        assert_eq!(template, "framework_precision");
    }
}
