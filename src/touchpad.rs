use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;

#[derive(Debug, Clone, Copy)]
pub enum TouchpadEvent {
    GestureStart,
    GestureDelta { dx: f64, dy: f64 },
    GestureEnd,
}

#[cfg(windows)]
mod platform {
    #![allow(unsafe_op_in_unsafe_fn)]

    use super::{Result, Sender, TouchpadEvent, anyhow};
    use log::info;
    use std::collections::{HashMap, HashSet};
    use std::ffi::OsStr;
    use std::iter;
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::thread;
    use winapi::shared::hidpi::{
        HIDP_BUTTON_CAPS, HIDP_CAPS, HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS, HidP_GetButtonCaps,
        HidP_GetCaps, HidP_GetUsageValue, HidP_GetUsages, HidP_GetValueCaps, HidP_Input,
        PHIDP_PREPARSED_DATA,
    };
    use winapi::shared::hidusage::{
        HID_USAGE_GENERIC_X, HID_USAGE_GENERIC_Y, HID_USAGE_PAGE_DIGITIZER, HID_USAGE_PAGE_GENERIC,
        USAGE,
    };
    use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
    use winapi::shared::ntdef::HANDLE;
    use winapi::shared::winerror::ERROR_CLASS_ALREADY_EXISTS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{
        CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA,
        GetMessageW, GetRawInputData, GetRawInputDeviceInfoW, GetSystemMetrics, GetWindowLongPtrW,
        MSG, PostMessageW, PostQuitMessage, RAWHID, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
        RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICENAME, RIDI_PREPARSEDDATA,
        RIM_TYPEHID, RegisterClassW, RegisterRawInputDevices, SM_CXVIRTUALSCREEN,
        SM_CYVIRTUALSCREEN, SetWindowLongPtrW, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_INPUT,
        WM_INPUT_DEVICE_CHANGE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSW, WS_OVERLAPPED,
    };

    const DIGITIZER_USAGE_TOUCH_PAD: USAGE = 0x05;
    const DIGITIZER_USAGE_TIP_SWITCH: USAGE = 0x42;
    const DIGITIZER_USAGE_CONTACT_ID: USAGE = 0x51;
    const DIGITIZER_USAGE_CONTACT_COUNT: USAGE = 0x54;

    pub struct ListenerHandle {
        hwnd: winapi::shared::windef::HWND,
        thread: Option<thread::JoinHandle<()>>,
    }

    #[derive(Debug, Clone, Copy)]
    struct PointF {
        x: f64,
        y: f64,
    }

    #[derive(Debug, Clone, Copy)]
    struct ValueLocator {
        usage_page: USAGE,
        usage: USAGE,
        link_collection: u16,
        logical_min: i32,
        logical_max: i32,
    }

    #[derive(Debug, Clone)]
    struct ContactLocator {
        link_collection: u16,
        x: Option<ValueLocator>,
        y: Option<ValueLocator>,
        contact_id: Option<ValueLocator>,
        tip_switch: bool,
    }

    struct TouchpadDevice {
        name: String,
        preparsed_data: Vec<u8>,
        contact_count: Option<ValueLocator>,
        contacts: Vec<ContactLocator>,
        max_usage_list_len: usize,
    }

    struct TouchpadState {
        sender: Sender<TouchpadEvent>,
        required_fingers: u8,
        sensitivity: f32,
        screen_width: f64,
        screen_height: f64,
        devices: HashMap<isize, TouchpadDevice>,
        active_centroid: Option<PointF>,
        pending_centroid: Option<PointF>,
        stable_activation_frames: u8,
        missing_frames: u8,
    }

    pub fn spawn_listener(
        sender: Sender<TouchpadEvent>,
        required_fingers: u8,
        sensitivity: f32,
    ) -> Result<ListenerHandle> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let thread = thread::spawn(move || unsafe {
            match create_listener_window(sender, required_fingers, sensitivity) {
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
            .map_err(|_| anyhow!("touchpad listener thread exited before initialization"))??;

        Ok(ListenerHandle {
            hwnd: hwnd as winapi::shared::windef::HWND,
            thread: Some(thread),
        })
    }

    impl Drop for ListenerHandle {
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

    unsafe fn create_listener_window(
        sender: Sender<TouchpadEvent>,
        required_fingers: u8,
        sensitivity: f32,
    ) -> Result<winapi::shared::windef::HWND> {
        let class_name = wide("ThreeWinDragTouchpadWindow");
        let instance = GetModuleHandleW(ptr::null());
        if instance.is_null() {
            return Err(anyhow!("failed to get module handle for touchpad listener"));
        }

        let mut wnd_class: WNDCLASSW = mem::zeroed();
        wnd_class.lpfnWndProc = Some(window_proc);
        wnd_class.hInstance = instance;
        wnd_class.lpszClassName = class_name.as_ptr();

        if RegisterClassW(&wnd_class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
            return Err(anyhow!("failed to register touchpad listener window class"));
        }

        let state = Box::new(TouchpadState {
            sender,
            required_fingers,
            sensitivity,
            screen_width: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as f64,
            screen_height: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as f64,
            devices: HashMap::new(),
            active_centroid: None,
            pending_centroid: None,
            stable_activation_frames: 0,
            missing_frames: 0,
        });
        let raw_state = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
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
            drop(Box::from_raw(raw_state));
            return Err(anyhow!("failed to create touchpad listener window"));
        }

        let raw_device = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_DIGITIZER,
            usUsage: DIGITIZER_USAGE_TOUCH_PAD,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        };

        if RegisterRawInputDevices(&raw_device, 1, mem::size_of::<RAWINPUTDEVICE>() as UINT) == 0 {
            return Err(anyhow!("failed to register raw touchpad input device"));
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
        hwnd: winapi::shared::windef::HWND,
        message: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                1
            }
            WM_INPUT => {
                if let Some(state) = state_from_hwnd(hwnd) {
                    let _ = state.handle_raw_input(lparam as _);
                }
                0
            }
            WM_INPUT_DEVICE_CHANGE => 0,
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            WM_NCDESTROY => {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TouchpadState;
                if !state_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(state_ptr));
                }
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    impl TouchpadState {
        unsafe fn handle_raw_input(&mut self, raw_input_handle: HANDLE) -> Result<()> {
            let mut size: UINT = 0;
            if GetRawInputData(
                raw_input_handle as _,
                RID_INPUT,
                ptr::null_mut(),
                &mut size,
                mem::size_of::<RAWINPUTHEADER>() as UINT,
            ) == u32::MAX
            {
                return Err(anyhow!("GetRawInputData size query failed"));
            }

            let mut buffer = vec![0u8; size as usize];
            if GetRawInputData(
                raw_input_handle as _,
                RID_INPUT,
                buffer.as_mut_ptr().cast(),
                &mut size,
                mem::size_of::<RAWINPUTHEADER>() as UINT,
            ) == u32::MAX
            {
                return Err(anyhow!("GetRawInputData payload query failed"));
            }

            let raw = &*(buffer.as_ptr() as *const RAWINPUT);
            if raw.header.dwType != RIM_TYPEHID {
                return Ok(());
            }

            let hdevice = raw.header.hDevice;
            if hdevice.is_null() {
                return Ok(());
            }

            let raw_hid: &RAWHID = raw.data.hid();
            let hid_size = raw_hid.dwSizeHid as usize;
            let hid_count = raw_hid.dwCount as usize;
            if hid_size == 0 || hid_count == 0 {
                return Ok(());
            }

            let data_len = hid_size * hid_count;
            let data = std::slice::from_raw_parts(raw_hid.bRawData.as_ptr(), data_len);

            let mut samples = Vec::new();
            for report in data.chunks_exact(hid_size) {
                let device = self.device_for(hdevice)?;
                if let Some(sample) = device.parse_report(report)? {
                    samples.push(sample);
                }
            }

            for sample in samples {
                self.consume_sample(sample);
            }

            Ok(())
        }

        unsafe fn device_for(&mut self, hdevice: HANDLE) -> Result<&mut TouchpadDevice> {
            let key = hdevice as isize;
            if !self.devices.contains_key(&key) {
                let device = TouchpadDevice::from_raw_device(hdevice)?;
                info!("registered touchpad raw input device: {}", device.name);
                self.devices.insert(key, device);
            }

            self.devices
                .get_mut(&key)
                .ok_or_else(|| anyhow!("touchpad device cache lookup failed"))
        }

        fn consume_sample(&mut self, sample: TouchSample) {
            if sample.finger_count == self.required_fingers as usize {
                if let Some(centroid) = sample.centroid {
                    self.missing_frames = 0;
                    if let Some(previous) = self.active_centroid {
                        let dx = ((centroid.x - previous.x)
                            * self.screen_width
                            * self.sensitivity as f64)
                            .clamp(-48.0, 48.0);
                        let dy = ((centroid.y - previous.y)
                            * self.screen_height
                            * self.sensitivity as f64)
                            .clamp(-48.0, 48.0);
                        if dx.abs() > 0.2 || dy.abs() > 0.2 {
                            let _ = self.sender.send(TouchpadEvent::GestureDelta { dx, dy });
                        }
                        self.active_centroid = Some(centroid);
                        return;
                    } else if let Some(previous) = self.pending_centroid {
                        self.stable_activation_frames =
                            self.stable_activation_frames.saturating_add(1);
                        if self.stable_activation_frames >= 2 {
                            let _ = self.sender.send(TouchpadEvent::GestureStart);
                            self.active_centroid = Some(centroid);
                            self.pending_centroid = None;
                            return;
                        }

                        self.pending_centroid = Some(PointF {
                            x: (previous.x + centroid.x) / 2.0,
                            y: (previous.y + centroid.y) / 2.0,
                        });
                        return;
                    }

                    self.pending_centroid = Some(centroid);
                    self.stable_activation_frames = 1;
                    return;
                }
            }

            self.pending_centroid = None;
            self.stable_activation_frames = 0;
            self.missing_frames = self.missing_frames.saturating_add(1);
            if self.missing_frames >= 2 && self.active_centroid.take().is_some() {
                let _ = self.sender.send(TouchpadEvent::GestureEnd);
            }
        }
    }

    struct TouchSample {
        finger_count: usize,
        centroid: Option<PointF>,
    }

    impl TouchpadDevice {
        unsafe fn from_raw_device(hdevice: HANDLE) -> Result<Self> {
            let name = get_device_name(hdevice).unwrap_or_else(|| String::from("unknown touchpad"));
            let preparsed_data = get_preparsed_data(hdevice)?;
            let ppd = preparsed_data.as_ptr() as PHIDP_PREPARSED_DATA;

            let mut caps: HIDP_CAPS = mem::zeroed();
            let status = HidP_GetCaps(ppd, &mut caps);
            if status != HIDP_STATUS_SUCCESS {
                return Err(anyhow!("HidP_GetCaps failed with status {status:#x}"));
            }

            let value_caps = get_value_caps(ppd, caps.NumberInputValueCaps)?;
            let button_caps = get_button_caps(ppd, caps.NumberInputButtonCaps)?;
            let max_usage_list_len = winapi::shared::hidpi::HidP_MaxUsageListLength(
                HidP_Input,
                HID_USAGE_PAGE_DIGITIZER,
                ppd,
            ) as usize;

            let mut contacts: HashMap<u16, ContactLocator> = HashMap::new();
            let mut contact_count = None;

            for cap in &value_caps {
                for locator in expand_value_cap(cap) {
                    if locator.usage_page == HID_USAGE_PAGE_DIGITIZER
                        && locator.usage == DIGITIZER_USAGE_CONTACT_COUNT
                    {
                        contact_count = Some(locator);
                    }

                    let entry =
                        contacts
                            .entry(locator.link_collection)
                            .or_insert_with(|| ContactLocator {
                                link_collection: locator.link_collection,
                                x: None,
                                y: None,
                                contact_id: None,
                                tip_switch: false,
                            });

                    match (locator.usage_page, locator.usage) {
                        (HID_USAGE_PAGE_GENERIC, HID_USAGE_GENERIC_X) => entry.x = Some(locator),
                        (HID_USAGE_PAGE_GENERIC, HID_USAGE_GENERIC_Y) => entry.y = Some(locator),
                        (HID_USAGE_PAGE_DIGITIZER, DIGITIZER_USAGE_CONTACT_ID) => {
                            entry.contact_id = Some(locator)
                        }
                        _ => {}
                    }
                }
            }

            for cap in &button_caps {
                for usage in expand_button_cap(cap) {
                    if usage.usage_page == HID_USAGE_PAGE_DIGITIZER
                        && usage.usage == DIGITIZER_USAGE_TIP_SWITCH
                    {
                        let entry = contacts.entry(usage.link_collection).or_insert_with(|| {
                            ContactLocator {
                                link_collection: usage.link_collection,
                                x: None,
                                y: None,
                                contact_id: None,
                                tip_switch: false,
                            }
                        });
                        entry.tip_switch = true;
                    }
                }
            }

            let mut contacts: Vec<ContactLocator> = contacts
                .into_values()
                .filter(|contact| {
                    contact.x.is_some()
                        && contact.y.is_some()
                        && (contact.tip_switch || contact.contact_id.is_some())
                })
                .collect();
            contacts.sort_by_key(|contact| contact.link_collection);

            if contacts.is_empty() {
                return Err(anyhow!(
                    "touchpad HID parser did not find finger collections for {name}"
                ));
            }

            Ok(Self {
                name,
                preparsed_data,
                contact_count,
                contacts,
                max_usage_list_len: max_usage_list_len.max(8),
            })
        }

        unsafe fn parse_report(&self, report: &[u8]) -> Result<Option<TouchSample>> {
            let ppd = self.preparsed_data.as_ptr() as PHIDP_PREPARSED_DATA;
            let mut contacts = Vec::new();
            let mut contact_ids = HashSet::new();

            for contact in &self.contacts {
                if !self.contact_is_active(contact, ppd, report)? {
                    continue;
                }

                let Some(x_locator) = contact.x else {
                    continue;
                };
                let Some(y_locator) = contact.y else {
                    continue;
                };

                let x_value = get_usage_value(ppd, &x_locator, report)?;
                let y_value = get_usage_value(ppd, &y_locator, report)?;
                let point = PointF {
                    x: normalize_value(x_value, x_locator.logical_min, x_locator.logical_max),
                    y: normalize_value(y_value, y_locator.logical_min, y_locator.logical_max),
                };

                if let Some(contact_id_locator) = contact.contact_id {
                    let contact_id = get_usage_value(ppd, &contact_id_locator, report)? as u32;
                    if !contact_ids.insert(contact_id) {
                        continue;
                    }
                }

                contacts.push(point);
            }

            let explicit_count = self
                .contact_count
                .and_then(|locator| get_usage_value(ppd, &locator, report).ok())
                .map(|value| value.max(0) as usize);

            let finger_count = explicit_count.unwrap_or(contacts.len()).max(contacts.len());
            if contacts.is_empty() {
                return Ok(Some(TouchSample {
                    finger_count,
                    centroid: None,
                }));
            }

            let sum_x: f64 = contacts.iter().map(|point| point.x).sum();
            let sum_y: f64 = contacts.iter().map(|point| point.y).sum();
            let centroid = PointF {
                x: sum_x / contacts.len() as f64,
                y: sum_y / contacts.len() as f64,
            };

            Ok(Some(TouchSample {
                finger_count,
                centroid: Some(centroid),
            }))
        }

        unsafe fn contact_is_active(
            &self,
            contact: &ContactLocator,
            ppd: PHIDP_PREPARSED_DATA,
            report: &[u8],
        ) -> Result<bool> {
            if contact.tip_switch {
                let mut usages = vec![0u16; self.max_usage_list_len];
                let mut usage_len = usages.len() as u32;
                let status = HidP_GetUsages(
                    HidP_Input,
                    HID_USAGE_PAGE_DIGITIZER,
                    contact.link_collection,
                    usages.as_mut_ptr(),
                    &mut usage_len,
                    ppd,
                    report.as_ptr() as *mut i8,
                    report.len() as u32,
                );

                if status == HIDP_STATUS_SUCCESS {
                    return Ok(usages[..usage_len as usize].contains(&DIGITIZER_USAGE_TIP_SWITCH));
                }
            }

            if let Some(contact_id) = contact.contact_id {
                return Ok(get_usage_value(ppd, &contact_id, report).is_ok());
            }

            Ok(false)
        }
    }

    #[derive(Clone, Copy)]
    struct ButtonLocator {
        usage_page: USAGE,
        usage: USAGE,
        link_collection: u16,
    }

    unsafe fn get_device_name(hdevice: HANDLE) -> Option<String> {
        let mut size: UINT = 0;
        if GetRawInputDeviceInfoW(hdevice, RIDI_DEVICENAME, ptr::null_mut(), &mut size) == u32::MAX
            || size == 0
        {
            return None;
        }

        let mut buffer = vec![0u16; size as usize];
        if GetRawInputDeviceInfoW(
            hdevice,
            RIDI_DEVICENAME,
            buffer.as_mut_ptr().cast(),
            &mut size,
        ) == u32::MAX
        {
            return None;
        }

        let end = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }

    unsafe fn get_preparsed_data(hdevice: HANDLE) -> Result<Vec<u8>> {
        let mut size: UINT = 0;
        if GetRawInputDeviceInfoW(hdevice, RIDI_PREPARSEDDATA, ptr::null_mut(), &mut size)
            == u32::MAX
            || size == 0
        {
            return Err(anyhow!("failed to query raw input preparsed data size"));
        }

        let mut buffer = vec![0u8; size as usize];
        if GetRawInputDeviceInfoW(
            hdevice,
            RIDI_PREPARSEDDATA,
            buffer.as_mut_ptr().cast(),
            &mut size,
        ) == u32::MAX
        {
            return Err(anyhow!("failed to fetch raw input preparsed data"));
        }

        Ok(buffer)
    }

    unsafe fn get_value_caps(
        ppd: PHIDP_PREPARSED_DATA,
        count: u16,
    ) -> Result<Vec<HIDP_VALUE_CAPS>> {
        let mut length = count;
        let mut caps = vec![mem::zeroed::<HIDP_VALUE_CAPS>(); count as usize];
        let status = HidP_GetValueCaps(HidP_Input, caps.as_mut_ptr(), &mut length, ppd);
        if status != HIDP_STATUS_SUCCESS {
            return Err(anyhow!("HidP_GetValueCaps failed with status {status:#x}"));
        }
        caps.truncate(length as usize);
        Ok(caps)
    }

    unsafe fn get_button_caps(
        ppd: PHIDP_PREPARSED_DATA,
        count: u16,
    ) -> Result<Vec<HIDP_BUTTON_CAPS>> {
        let mut length = count;
        let mut caps = vec![mem::zeroed::<HIDP_BUTTON_CAPS>(); count as usize];
        let status = HidP_GetButtonCaps(HidP_Input, caps.as_mut_ptr(), &mut length, ppd);
        if status != HIDP_STATUS_SUCCESS {
            return Err(anyhow!("HidP_GetButtonCaps failed with status {status:#x}"));
        }
        caps.truncate(length as usize);
        Ok(caps)
    }

    unsafe fn expand_value_cap(cap: &HIDP_VALUE_CAPS) -> Vec<ValueLocator> {
        if cap.IsRange != 0 {
            let range = cap.u.Range();
            (range.UsageMin..=range.UsageMax)
                .map(|usage| ValueLocator {
                    usage_page: cap.UsagePage,
                    usage,
                    link_collection: cap.LinkCollection,
                    logical_min: cap.LogicalMin,
                    logical_max: cap.LogicalMax,
                })
                .collect()
        } else {
            vec![ValueLocator {
                usage_page: cap.UsagePage,
                usage: cap.u.NotRange().Usage,
                link_collection: cap.LinkCollection,
                logical_min: cap.LogicalMin,
                logical_max: cap.LogicalMax,
            }]
        }
    }

    unsafe fn expand_button_cap(cap: &HIDP_BUTTON_CAPS) -> Vec<ButtonLocator> {
        if cap.IsRange != 0 {
            let range = cap.u.Range();
            (range.UsageMin..=range.UsageMax)
                .map(|usage| ButtonLocator {
                    usage_page: cap.UsagePage,
                    usage,
                    link_collection: cap.LinkCollection,
                })
                .collect()
        } else {
            vec![ButtonLocator {
                usage_page: cap.UsagePage,
                usage: cap.u.NotRange().Usage,
                link_collection: cap.LinkCollection,
            }]
        }
    }

    unsafe fn get_usage_value(
        ppd: PHIDP_PREPARSED_DATA,
        locator: &ValueLocator,
        report: &[u8],
    ) -> Result<i32> {
        let mut value: u32 = 0;
        let status = HidP_GetUsageValue(
            HidP_Input,
            locator.usage_page,
            locator.link_collection,
            locator.usage,
            &mut value,
            ppd,
            report.as_ptr() as *mut i8,
            report.len() as u32,
        );

        if status == HIDP_STATUS_SUCCESS {
            return Ok(value as i32);
        }

        if status == winapi::shared::hidpi::HIDP_STATUS_USAGE_NOT_FOUND {
            return Err(anyhow!("usage not found"));
        }

        Err(anyhow!(
            "HidP_GetUsageValue failed for usage page {:x} usage {:x} with status {status:#x}",
            locator.usage_page,
            locator.usage
        ))
    }

    fn normalize_value(value: i32, logical_min: i32, logical_max: i32) -> f64 {
        let span = (logical_max - logical_min).max(1) as f64;
        ((value - logical_min) as f64 / span).clamp(0.0, 1.0)
    }

    unsafe fn state_from_hwnd(
        hwnd: winapi::shared::windef::HWND,
    ) -> Option<&'static mut TouchpadState> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TouchpadState;
        if ptr.is_null() { None } else { Some(&mut *ptr) }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{Result, Sender, TouchpadEvent, anyhow};

    pub struct ListenerHandle;

    pub fn spawn_listener(
        _sender: Sender<TouchpadEvent>,
        _required_fingers: u8,
        _sensitivity: f32,
    ) -> Result<ListenerHandle> {
        Err(anyhow!(
            "three-finger touchpad input is only implemented on Windows"
        ))
    }
}

pub use platform::{ListenerHandle, spawn_listener};
