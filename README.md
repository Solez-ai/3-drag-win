# 3-win-drag

<p align="center">
  <img src="logo.png" alt="3-win-drag logo" width="180" />
</p>

Windows laptops have these big fancy Precision Touchpads but you can't just grab a window and drag it from anywhere like you can on a Mac. That's what this fixes.

Put three fingers on your touchpad and drag. That's it. No title bar reaching, no alt+tab nonsense. Just drag.

Works on **Windows 10 and Windows 11**. That's it for now. Linux support is in the works on the experimental branch.

## What it does

- Three-finger drag to move windows from anywhere on screen
- Three-finger drag to grab and drag things inside apps (files, tabs, images)
- Runs in your system tray with a little icon
- Starts automatically when you log in
- Lets you tweak sensitivity, deadzone, smoothing, and more

## How to use it

1. Download the latest `3-win-drag-windows-x64.zip` from the Releases page
2. Extract it somewhere
3. Run `3-win-drag.exe`
4. Put three fingers on your touchpad and drag any window

You'll see the icon in your system tray. Right click it to change settings or turn things off.

## Settings you can change

Open the settings window from the tray menu. You can tune:

- **Gesture action** - Window move (moves the whole window) or mouse drag (grabs things inside apps)
- **Sensitivity** - How much the window moves when you drag
- **Deadzone** - Ignores tiny finger jitter so windows dont shake
- **Smoothing** - Makes movement feel less jerky
- **Finger count** - How many fingers to trigger the drag (default is 3)
- **Fullscreen guard** - Blocks dragging on fullscreen windows so games dont get messed up

There are also presets for different laptop brands like Lenovo, Dell, HP, ASUS, Surface, and more. The app tries to detect your hardware and pick the best one.

## Downloads

| File | What it is |
|------|------------|
| `3-win-drag-windows-x64.zip` | Windows 10 and 11, 64-bit. Just extract and run. |

## Building from source

You need the Rust toolchain (stable) and a C++ compiler (MSVC or MinGW).

```powershell
cargo build --release
```

The binary will be at `target/release/3-win-drag.exe`.

## How it works under the hood

I wont bore you with the full architecture, but the short version:

- Rust does everything (input handling, state, tray, config, drag logic)
- C++ handles the Windows-specific window management stuff via FFI
- It reads your touchpad at the raw HID level, not through some high level API
- Config lives in `%LOCALAPPDATA%\solez-ai\3-win-drag\data\config.json`

## Known issues

- Windows has its own three-finger gestures for things like switching desktops. If those get in the way, turn them off in Windows Settings > Bluetooth & devices > Touchpad.
- Some apps with custom title bars might not play nice. Games and fullscreen stuff are blocked on purpose.
- If you run an older version at the same time, the new one will tell you its already running. Check your system tray.

## Why not Linux?

Linux support (X11 and Wayland on Hyprland, Sway, KDE) is on the `experimental` branch. Its mostly working but not stable enough for a public launch yet. If you want to help test it, check out that branch.

## Who made this

**Samin Yeasar**

- GitHub: https://github.com/solez-ai
- X: https://x.com/Solez_None

## License

MIT. See LICENSE for the full text.
