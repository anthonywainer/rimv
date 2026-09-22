---
name: linux-native
description: "Use for RimV Linux desktop integration, PipeWire/PulseAudio/ALSA, desktop UI, Wayland/X11, XDG paths and Linux packaging. Not for portable Rust logic alone."
---

# Linux Native Development

## Goal
Support realistic Linux desktop environments without hard-coding one distribution or audio server.

## Identify the actual environment
Before editing, determine the target distro/version, architecture, desktop environment, Wayland/X11 status, audio stack and packaging format where they affect behavior. Do not assume GNOME, KDE, X11 or PulseAudio universally.

## Implementation workflow
1. Find the native Linux adapter and the shared Rust interface it implements.
2. Inspect capability requirements rather than branching solely on distro names.
3. Use XDG conventions for config, cache, data and runtime files where appropriate.
4. For native UI, respect the chosen toolkit's appearance, font scaling, keyboard focus, menus and notifications.
5. For audio, use the user's configured stack (PipeWire, PulseAudio compatibility or ALSA when appropriate); handle disconnects and permission/portal responses.
6. Preserve cancellation, resource cleanup and startup/shutdown across display/audio session changes.
7. Validate at least one concrete environment and record limitations.

## Desktop integration
System tray/status protocols differ across desktop environments; use a documented fallback when support is missing. File access and capture permissions may be portal-mediated in sandboxed packages. Do not hard-code `$HOME` paths or treat `/tmp` as persistent storage.

## Packaging
Select formats justified by distribution goals (Flatpak, AppImage, native packages or archive). Check native library availability, runtime dependencies, sandbox permissions, architecture and reproducible version labels. Do not modify all formats for a change affecting only one.

## Tests
Keep portable logic under cross-platform tests and isolate environment-dependent integration checks. Report the distro, desktop, display stack and audio server actually tested.

## Exit
User-visible errors have recovery instructions; no unintended environment assumptions remain; integration and packaging assumptions are documented.
