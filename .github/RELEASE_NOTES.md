## What is this?

3-win-drag brings the macOS three-finger drag experience to Windows and Linux. Put three fingers on your touchpad and drag any window from anywhere on screen. No title bar needed.

## Downloads

- **3-win-drag-windows-x64.zip** - Windows 10 and 11. Unzip and run 3-win-drag.exe.
- **3-win-drag-linux-x86_64.tar.gz** - Linux x86_64 with glibc. Tested on Ubuntu 22.04 and 24.04.
- **3-win-drag-linux-x86_64-musl.tar.gz** - Fully static Linux x86_64 binary. Works on minimal containers and distros without glibc.
- **3-win-drag-linux-aarch64.tar.gz** - Linux ARM64. For Raspberry Pi, Apple Silicon VMs, etc.

## What works

- Three-finger gesture detection on Windows Precision Touchpads and Linux evdev
- Window move, mouse drag, and cursor control
- System tray integration with enable/disable toggles
- Auto-start on sign in (Windows registry / XDG autostart)
- Multi-monitor aware positioning
- Fullscreen and minimized window guards
- JSON config with live settings

## Linux support

- X11: Pure Rust x11rb backend works on any EWMH-compliant window manager (GNOME, KDE, i3, etc.)
- Wayland: Compositor-specific backends for Hyprland, Sway, and KDE Plasma
- Touchpad input via evdev kernel interface
- XDG autostart and PID file for single instance

## Known issues

- Manjaro Linux: Works but needs optimization. Some touchpad devices may not be detected correctly on first try. Run with --help for troubleshooting.
- Wayland mouse click simulation only works on Hyprland (via fakeinput). Sway and KDE use WindowMove gesture only.
- GNOME Wayland is not supported. Use an X11 session or switch to Hyprland, Sway, or KDE.
- Windows three-finger gestures may conflict with built-in Windows gesture mappings. Disable them in Windows Settings.
