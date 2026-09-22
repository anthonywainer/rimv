---
name: windows-native
description: "Use for RimV Windows desktop UI, WinUI/Win32 interoperability, system tray, WASAPI, installer behavior and Windows-specific Rust bindings."
---

# Windows Native Development

## Goal
Deliver a Windows-native RimV experience using an appropriate Windows technology stack, without presuming the project already uses a specific framework.

## Before implementation
1. Identify the actual Windows app host and toolchain in the repository. Do not silently create a WinUI project if another UI technology is established.
2. Check supported Windows versions and processor architectures.
3. Identify what belongs in shared Rust code versus Windows adapter/GUI.
4. For UI work, load the RimV design system and platform guidance when available.

## Native integration
- Prefer supported Windows desktop APIs; use Win32/COM where necessary rather than forcing wrappers for simple capabilities.
- Document thread apartment requirements (`STA`/`MTA`) where COM or UI integration depends on them.
- Handle UTF-16 conversion and ownership explicitly at FFI boundaries.
- Close native handles in every success/failure path; do not pass borrowed pointers beyond their valid scope.
- Use familiar Windows tray, notifications, title-bar, focus, keyboard and file-dialog conventions.

## Audio and privacy
For WASAPI/device work, verify sample format negotiation, endpoint role, device removal, default-device changes and microphone privacy settings. Keep VAD/resampling and transcription policy in shared audio crates.

## Distribution
Choose packaging based on project requirements (e.g. installer/MSIX), considering architecture, dependent DLLs, signing, upgrade behavior, uninstallation and retained user data. Do not claim packaging is complete until the install/uninstall flow was actually checked.

## Accessibility
Verify keyboard-only navigation, Narrator names, visible focus, scaling, high-contrast mode and reduced-motion settings.

## Validation matrix
Record tested Windows version, x64/ARM64 target if relevant, UI host, capture device path and installer variant. If you cannot run Windows locally, compile/check cross-target code where possible and explicitly mark on-device behavior unverified.
