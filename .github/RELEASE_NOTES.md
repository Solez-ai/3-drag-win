## 3-win-drag v0.1.1

Ever wanted to move a window by grabbing it from anywhere, not just the title bar? That's what this does. Put three fingers on your touchpad and drag. Works on Windows and Linux.

I built this because I got tired of reaching for tiny title bars. macOS has three-finger drag built in. Windows doesn't. So I made it.

### Downloads

Pick your platform:

- **3-win-drag-windows-x64.zip** -- Windows 10 and 11. Unzip and run 3-win-drag.exe.
- **3-win-drag-linux-x86_64.tar.gz** -- Linux x86_64 with glibc. Tested on Ubuntu 22.04 and 24.04.
- **3-win-drag-linux-x86_64-musl.tar.gz** -- Linux x86_64, statically linked. Works on distros without glibc.
- **3-win-drag-linux-aarch64.tar.gz** -- Linux ARM64. For Raspberry Pis, Apple Silicon VMs, etc.

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
