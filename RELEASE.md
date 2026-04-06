# GitHub Release Template

Use the exact structure below for the GitHub release entry. Replace `X.Y.Z` with the actual version number before publishing.

Release title:

```text
3-win-drag vX.Y.Z
```

Release description:

```md
![3-win-drag logo](https://raw.githubusercontent.com/Solez-ai/3-drag-win/main/three-win-drag/logo.png)

# 3-win-drag vX.Y.Z

This was made by Samin Yeasar.

This is an open source project on GitHub: https://github.com/Solez-ai/3-drag-win

MIT Licensed.

## What This Release Includes

- Windows installer wizard for Windows 10 and Windows 11
- Native desktop settings window
- Three-finger drag and drop enabled by default through the `drag_drop_precise` profile
- Background startup behavior so the tool remains available after sign-in
- Updated touchpad tuning templates and live configuration support

## Installation

1. Download `3-win-drag-setup-X.Y.Z.exe` from the Assets section below.
2. Run the installer wizard.
3. When the installer instructs you to do so, open Windows Settings and disable the built-in three-finger touchpad gestures.
4. Finish the installation and allow the app to launch.
5. Use the tray icon to open the native settings window if you want to adjust sensitivity, smoothing, or templates.

## Notes

- The default install profile is `drag_drop_precise`.
- The application is intended for Windows Precision Touchpad hardware.
- If Windows still responds to three-finger gestures, re-check the touchpad settings and make sure those built-in actions are disabled.

## Assets

- `3-win-drag-setup-X.Y.Z.exe`: the Windows installer wizard
- `SHA256SUMS.txt`: optional checksum file if you publish one
```

Recommended asset names:

```text
3-win-drag-setup-X.Y.Z.exe
SHA256SUMS.txt
```

Publishing checklist:

1. Run `powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1`
2. Confirm the installer exists in `dist\installer\`
3. Confirm the installer launches and shows the touchpad-gesture reminder page
4. Confirm `Open settings` opens the native desktop settings window after installation
5. Upload the installer to the GitHub release
6. Paste the release description above into the release body
