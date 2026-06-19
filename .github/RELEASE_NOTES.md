## 3-win-drag v0.1.2

Ever wanted to move a window by grabbing it from anywhere, not just the title bar? That's what this does. Put three fingers on your touchpad and drag. Works on Windows and Linux.

I built this because I got tired of reaching for tiny title bars. macOS has three-finger drag built in. Windows doesn't. So I made it.

### Downloads

Pick your platform:

- **3-win-drag-windows-x64.exe** -- Windows 10 and 11. Download and run.
- **3-win-drag-linux-x86_64** -- Linux x86_64 with glibc. Tested on Ubuntu 22.04 and 24.04. Make executable with `chmod +x`.

### What's new in v0.1.2

- Fixed all compilation errors for Rust 1.96.0 (edition 2024)
- Updated all dependencies to latest compatible versions
- Better Linux backend support across X11 and Wayland compositors
- Arch Linux compatibility improvements

### What works

- Three-finger drag on Windows Precision Touchpads and Linux evdev
- Window move, mouse drag, and cursor control
- System tray with enable/disable toggle
- Auto-start on login (Windows registry / XDG autostart)
- Multi-monitor support
- Blocks dragging on fullscreen and minimized windows
- Settings in a JSON file, applies live

### Linux

- X11: Works on any EWMH window manager (GNOME, KDE, i3, etc.)
- Wayland: Works on Hyprland, Sway, and KDE Plasma (each uses its own IPC)
- Touchpad input from evdev
- XDG autostart and PID file so only one instance runs

### Known issues

- Manjaro: Works but the touchpad detection needs tuning. Run with --help if it doesn't pick up your device.
- Wayland mouse clicks only work on Hyprland. Sway and KDE can move windows but not simulate clicks.
- GNOME Wayland is not supported. Use X11 or switch to Hyprland/Sway/KDE.
- Windows built-in three-finger gestures might conflict. Turn them off in Windows Settings.
- Cross-compiled binaries for musl/aarch64/armv7/riscv64 are built via CI on tagged releases.
