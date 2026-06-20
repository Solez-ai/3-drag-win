## 3-win-drag v0.1.2

Ever wanted to move a window by grabbing it from anywhere, not just the title bar? That's what this does. Put three fingers on your touchpad and drag. Works on Windows 10 and 11.

I built this because I got tired of reaching for tiny title bars. macOS has three-finger drag built in. Windows doesn't. So I made it.

### What's new in v0.1.2

This is the first public release. Everything is set up for Windows.

- Three-finger drag works on any Precision Touchpad
- Choose between window move mode or mouse drag mode
- System tray with enable/disable toggle
- Auto start on login
- Full settings window with sensitivity, deadzone, smoothing controls
- Presets for Lenovo, Dell, HP, ASUS, Surface, MSI, Framework, and more
- Detects your hardware and picks the best template
- Multi-monitor support
- Blocks dragging on fullscreen windows so games dont get messed up
- Minimized windows and unsupported windows are skipped

### Downloads

| File | What it is |
|------|------------|
| `3-win-drag-windows-x64.zip` | Windows 10 and 11, 64-bit. Extract and run. |

### Known issues

- Windows built-in three-finger gestures might conflict. Turn them off in Windows Settings under Bluetooth & devices > Touchpad if things feel off.
- Some apps with custom title bars might not play nice.
- If you already have 3-win-drag running and try to start it again, it will tell you and exit.

### Linux

Linux support is on the experimental branch. Its working on X11, Hyprland, Sway, and KDE but needs more testing before a public launch.
