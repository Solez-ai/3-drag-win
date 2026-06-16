# 3-win-drag

<p align="center">
  <img src="logo.png" alt="3-win-drag logo" width="180" />
</p>

3-win-drag is a professional background utility that brings true three-finger touchpad window dragging to your desktop with a native, low-latency feel. The application is designed to let users move standard desktop windows from anywhere on screen instead of depending on the title bar, while remaining lightweight enough to stay active for the entire session with minimal overhead.

**Supported platforms:**
- **Windows 10/11** — Full native support with Windows Precision Touchpad HID input and C++ Win32 window management
- **Linux (X11)** — Full support via evdev multi-touch input and x11rb X11 window management

This repository implements the architecture with a Rust application core and platform-specific window-control layers. Rust owns orchestration, input handling, state, configuration, startup integration, tray behavior, and drag logic.

## Current Delivery Scope

The application is production-oriented on both Windows and Linux (X11).

| Feature | Windows | Linux (X11) |
|---|---|---|
| Touchpad input | Windows Raw Input + HID parsing | evdev multi-touch device |
| Window management | C++ Win32 FFI | x11rb (X11 protocol, Rust) |
| Mouse simulation | Win32 SendInput | XTest extension |
| System tray | Native Win32 tray | tray-item + GTK/libappindicator |
| Auto-start | Windows registry | XDG autostart (.desktop file) |
| Single instance | Named mutex | PID file in /tmp |
| Settings window | Native Win32 window | No-op (terminal config editing) |

## Implemented Features

- Silent background execution with no visible console window.
- System tray presence using the project logo as the application icon.
- Native Windows settings window for live configuration changes.
- Touchpad templates with vendor-aware recommendations and manual switching.
- Three-finger touchpad gesture detection (Windows HID / Linux evdev).
- Relative window movement based on touchpad centroid movement and an anchor window position.
- Deadzone filtering to suppress jitter from micro-movements.
- Optional smoothing support through a configurable interpolation factor.
- Multi-monitor aware drag movement.
- DPI-awareness bootstrap during process startup (Windows only).
- Maximized-window restore handling before drag movement begins.
- Full-screen and unsupported window avoidance.
- Minimized-window rejection.
- Automatic startup registration (Windows registry / XDG autostart).
- Persistent JSON configuration storage.
- Persistent file-based logging for background diagnostics.
- Separate Rust and platform-specific window-control layers.
- Release profile stripping for smaller production binaries.

## Runtime Behavior

At startup the application performs the following sequence:

1. Resolve and create its application data directories.
2. Initialize file logging.
3. Hide any attached console window (Windows) or run as a background process (Linux).
4. Enable DPI awareness (Windows) or initialize X11 connection (Linux).
5. Load configuration from disk or create a default configuration on first launch.
   The default first-run profile is `drag_drop_precise`.
6. Synchronize the auto-start setting.
7. Create the tray icon and its menu.
8. Start the touchpad input listener (Windows HID / Linux evdev).
9. Enter a controller loop that reacts to input events and tray commands.

During a drag session:

1. A three-finger touchpad gesture begins.
2. The current foreground window is validated and, if necessary, restored from maximized state.
3. The original window position becomes the anchor point.
4. Touchpad centroid deltas are converted into screen-space drag movement.
5. Deadzone and smoothing logic are applied before dispatching movement.
6. The native layer moves the target window with `SetWindowPos`.
7. Releasing the three-finger gesture ends the session immediately.

## Architecture

### Rust application layer

The Rust layer is responsible for:

- application startup and lifecycle
- background event loop
- global touchpad input capture
- session state and drag logic
- configuration persistence
- logging
- tray interactions
- auto-start management
- platform-specific backends via conditional compilation

Primary Rust modules:

- `src/app.rs`: controller loop, drag session state, input orchestration, tray command handling
- `src/config.rs`: configuration schema and application paths
- `src/autostart.rs`: startup registration helpers using `auto-launch`
- `src/tray.rs`: tray icon and menu wiring
- `src/logging.rs`: file-based logger bootstrap
- `src/touchpad.rs`: Windows: Raw Input HID parsing; Linux: evdev multi-touch device
- `src/ffi.rs`: Windows: native Win32/C++ bridge; Linux: x11rb X11 window management
- `src/main.rs`: process entry point and fatal startup handling

### Platform-specific layers

**Windows (C++):**
The C++ layer is intentionally narrow. It exposes a small set of externally callable functions:

- `drag_bootstrap_process` — DPI awareness setup
- `drag_prepare_foreground_window` — Window validation and maximized restore
- `drag_move_window` — SetWindowPos dispatch
- `drag_window_is_valid` — Window handle validation
- `drag_get_cursor_position` — System cursor position

Source files: `cpp/drag.h`, `cpp/drag.cpp`

**Linux (Rust x11rb):**
On Linux, all window management is implemented natively in Rust using the x11rb crate:

- `linux_prepare_foreground_window` — Queries `_NET_ACTIVE_WINDOW`, validates geometry, rejects fullscreen and minimized windows
- `linux_move_window` — Sends `_NET_MOVE_WINDOW` client message (EWMH standard)
- `linux_window_is_valid` — Checks if X11 window still exists
- `linux_current_cursor_position` — X11 QueryPointer
- `linux_button_press/linux_button_release` — XTest extension for mouse simulation

## Project Layout

```text
three-win-drag/
├── .cargo/config.toml
├── build.rs
├── Cargo.toml
├── LICENSE
├── README.md
├── logo.png
├── rust-toolchain.toml
├── cpp/
│   ├── drag.cpp
│   └── drag.h
└── src/
    ├── app.rs
    ├── autostart.rs
    ├── commands.rs
    ├── config.rs
    ├── ffi.rs
    ├── logging.rs
    ├── main.rs
    ├── settings_ui.rs
    ├── single_instance.rs
    ├── touchpad.rs
    └── tray.rs
```

## Configuration

The application stores configuration at:

**Windows:**
```text
%LOCALAPPDATA%\solez-ai\3-win-drag\data\config.json
```

**Linux:**
```text
~/.local/share/3-win-drag/config.json
```

Default configuration:

```json
{
  "enabled": true,
  "launch_at_startup": true,
  "touchpad_profile": "drag_drop_precise",
  "gesture_action": "mouse_drag",
  "gesture_finger_count": 3,
  "touchpad_sensitivity": 0.68,
  "deadzone_pixels": 8,
  "minimum_update_interval_ms": 4,
  "smoothing_factor": 0.8,
  "ignore_fullscreen_windows": true
}
```

Configuration fields:

- `enabled`: master switch for drag behavior
- `launch_at_startup`: controls Windows startup registration
- `touchpad_profile`: saved template/profile identifier shown in the settings UI
- `gesture_action`: `window_move` for direct whole-window movement, `mouse_drag` for native drag-and-drop inside apps
- `gesture_finger_count`: number of simultaneous touch contacts required to begin dragging
- `touchpad_sensitivity`: multiplier applied to the touchpad centroid delta before window movement
- `deadzone_pixels`: ignores tiny gesture shifts that create visible jitter
- `minimum_update_interval_ms`: minimum spacing between tiny movement updates
- `smoothing_factor`: `1.0` means direct movement; lower values apply interpolation
- `ignore_fullscreen_windows`: retains guardrails for games and full-screen applications

If the configuration file becomes invalid JSON, the application preserves a backup as `config.invalid.json` and recreates a valid default configuration.

## Tray Menu

The tray menu exposes operational controls appropriate for a background utility:

- Enable dragging
- Disable dragging
- Enable auto start
- Disable auto start
- Open settings
- Open data folder
- Open log folder
- Exit

The tray icon uses the project logo resource generated from `logo.png`.

## Settings Window

The app includes a native Windows settings window so users can tune behavior without editing JSON manually or leaving the desktop application experience.

- Open it from the tray menu with `Open settings`
- The window reuses the application icon and opens as a normal desktop window
- Fresh installs default to the `drag_drop_precise` profile so native drag-and-drop works immediately
- Changes apply live while the app is running
- Template application writes the config file and updates the running app

The settings window exposes:

- action mode selection between `window_move` and `mouse_drag`
- sensitivity, deadzone, smoothing, and update interval controls
- finger-count and fullscreen-ignore options
- touchpad templates for common laptop families and touchpad vendors
- hardware detection and a recommended template based on manufacturer/model/touchpad identity

## Windows Gesture Note

Windows Precision Touchpad systems may already have operating-system-level three-finger gestures assigned to task switching, search, or virtual desktops. 3-win-drag reads the touchpad at the raw HID layer, but if the built-in Windows gesture mappings interfere on a specific laptop, disable or reduce the stock three-finger touchpad gestures in Windows settings so 3-win-drag can own that gesture consistently.

## Build Toolchain

This repository is configured to build with:

- Rust toolchain: `stable`
- C++ compiler: MinGW `g++` (Windows) or `g++` (Linux for GTK tray deps)

Windows-specific:
- Resource compiler: `rc.exe`
- MSVC or MinGW toolchain

Linux-specific:
- X11 development libraries: `libx11-dev`, `libxtst-dev`
- GTK3 development libraries: `libgtk-3-dev`, `libappindicator3-dev` (for tray)

## Build and Run

### Prerequisites

**Linux (Ubuntu/Debian):**
```bash
sudo apt install libx11-dev libxtst-dev libgtk-3-dev libappindicator3-dev
```

**Linux (Fedora):**
```bash
sudo dnf install libX11-devel libXtst-devel gtk3-devel libappindicator-gtk3-devel
```

### Debug build

```bash
cargo build
```

### Release build

```bash
cargo build --release
```

### Build the Windows installer (Windows only)

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1
```

Installer output:
```text
dist\installer\3-win-drag-setup-<version>.exe
```

### Run the executable

**Windows:**
```powershell
.\target\release\3-win-drag.exe
```

**Linux:**
```bash
./target/release/3-win-drag
```

Note on Cargo naming:
- Cargo package names cannot start with a digit.
- The internal package name is therefore `three-win-drag`.
- The produced binary name remains `3-win-drag`, which matches the intended product identity.

## Cargo and Build Configuration

The project includes a production-oriented configuration:

- package metadata in `Cargo.toml`
- stripped release binaries through `[profile.release] strip = true`
- Linux dependencies (`x11rb`, `evdev`) in platform-specific section
- Windows dependencies (`winapi`, `windows`) in platform-specific section
- Cross-platform crates: `tray-item`, `auto-launch`

Build behavior:
- `build.rs` converts `logo.png` into platform-appropriate icon format
- `build.rs` compiles the C++ backend on Windows if available; falls back to pure-Rust WinAPI

## Logging and Diagnostics

Log file location:

**Windows:**
```text
%LOCALAPPDATA%\solez-ai\3-win-drag\data\logs\3-win-drag.log
```

**Linux:**
```text
~/.local/share/3-win-drag/logs/3-win-drag.log
```

## Window Handling Details

The drag engine is opinionated about what it should and should not move.

Supported behavior:
- standard visible desktop windows
- foreground windows on single- or multi-monitor setups
- restored maximized windows that can be transitioned into a drag state

Rejected or guarded contexts:
- minimized windows
- full-screen or likely borderless full-screen windows
- unsupported windows that fail geometry or monitor inspection

## Performance Design

Performance-sensitive choices in this implementation include:
- event-driven global input instead of polling loops
- no blocking work inside the drag update path beyond essential native calls
- deadzone filtering for noise suppression
- configurable update interval limits for tiny movements
- a minimal native ABI surface
- release-time stripping for smaller production binaries

## Auto-Start Behavior

Startup registration is handled programmatically through `auto-launch`.
- **Windows:** Registry entry in HKCU\Software\Microsoft\Windows\CurrentVersion\Run
- **Linux:** XDG autostart `.desktop` file at `~/.config/autostart/3-win-drag.desktop`

Defaults to enabling auto-start on first run; the tray menu can toggle it.

## Security and Operational Considerations

3-win-drag is a background system utility. That means operational discipline matters:
- It observes global input events.
- It writes startup settings when auto-start is enabled.
- It writes logs and configuration to the user profile.
- It intentionally avoids moving full-screen windows to reduce interference with games and immersive applications.
- The project does not inject into other processes, does not patch system files, and does not require elevated privileges.

## Known Limitations

- **Linux:** X11 only. Wayland is not yet supported.
- **Linux:** Settings window unavailable; configure via JSON file or tray menu.
- **Linux:** Maximized-window restore not implemented.
- Three-finger gesture quality depends on touchpad hardware and driver quality.
- Some highly customized or protected application windows may not behave normally.

## Future Directions

The codebase is intentionally structured so later work can extend it without rewriting the core:
- Wayland support (ext-foreign-toplevel compositor protocol)
- Native GTK settings window for Linux
- Maximized window restore on Linux
- richer trigger-key and sensitivity configuration
- per-application ignore lists
- more advanced smoothing profiles
- signed distribution packaging

## Verification Performed

**Windows:**
- `cargo build`
- `cargo check`

**Linux:**
- `cargo check` verified compilation succeeds

## Creator

**Samin Yeasar**

- GitHub: https://github.com/solez-ai
- X: https://x.com/Solez_None
- Portfolio: https://solez.vecel.app

## License

This project is released under the MIT License. See [`LICENSE`](LICENSE) for the full text.
