---
name: cross-platform
description: "Use when a RimV change must work across Apple, Windows, Linux and/or Web, especially shared Rust contracts, platform adapters, capabilities and CI parity. Not needed for isolated platform UI polish."
---

# Cross-Platform Engineering

## Goal
Keep a portable RimV core with native experiences and explicit capability-based boundaries.

## Classify the change
- **Portable domain:** audio transforms, model lifecycle, protocol types, business state.
- **Platform adapter:** microphone permissions, capture APIs, windows, notifications, menu/tray, file dialogs.
- **Presentation:** SwiftUI/AppKit, Windows UI, Linux toolkit, TypeScript web UI.
- **Packaging:** runtime dependencies, installers, distribution metadata.

## Design sequence
1. Identify which observable behavior must match across platforms and which behavior should remain native.
2. Define the shared contract (types, error codes, async/cancel semantics, time units, serialization).
3. Keep platform-specific dependencies behind the adapter. Avoid conditional compilation spread throughout domain modules.
4. Model capability differences explicitly: unsupported/not permitted/not installed/device missing; do not collapse them into one generic failure.
5. Keep UI semantics consistent, but use platform-appropriate navigation, menus, keyboard shortcuts and permission recovery.
6. Plan platform-specific tests independently of portable core tests.

## Compatibility matrix template
| Capability | macOS/iOS | Windows | Linux | Web |
|---|---|---|---|---|
| Microphone permission | Native | OS privacy | Portal/stack | Browser permission |
| Native audio backend | Verify project | WASAPI if used | PipeWire/ALSA if used | Browser APIs if used |
| System integration | Menu/status | Tray | Desktop dependent | Browser constrained |
| Model runtime | Inspect bundled backend | Check DLL/ABI | Check SO/ABI | Only if supported |

This matrix is a review template, not an assertion that every feature is already implemented.

## Contract checklist
Check path separators, case sensitivity, encoding, filesystem permissions, native binary architectures, dynamic library loading, clock assumptions and portability of serialized timestamps.

## Validation
Run portable Rust tests first. Then run the actual platform-specific build/integration checks available. Do not infer Windows/Linux success from a passing macOS build or vice versa. Document untested targets.
