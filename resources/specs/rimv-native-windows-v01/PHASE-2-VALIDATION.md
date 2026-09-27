# RimV Native Windows v0.1 — Phase 2 Implementation and Validation

**Status:** Native shell implementation prepared; Windows runtime validation is pending.
**Scope:** Native WinUI 3 shell only. This phase does not add an installer, alter the macOS UI, or replace `apps/windows` (the existing HTML/Tauri implementation).

## Architecture delivered

- New standalone WinUI 3 project: `apps/windows-native/RimV.Windows.csproj`. It produces an unpackaged `WinExe`, targets x64 and ARM64, and has no WebView, browser, localhost server, or console frontend.
- A hidden WinUI shell HWND owns a native Windows notification-area icon through `Shell_NotifyIconW`. Tray activation toggles a compact, always-on-top WinUI popup positioned inside the active monitor work area. Popup dismissal hides the shell; it does not stop capture. Quit is explicit.
- Independent native windows are provided for languages, models, recordings, settings, and transcript/playback. The transcript window does not open when capture starts and closing it does not stop a session.
- `Services/CoreClient.cs` is a thin P/Invoke client for the Phase 1 versioned `rimv-ffi` C ABI. `Services/AppCoordinator.cs` serializes requests through the client and distributes real core snapshots/events to the UI. Local preferences are limited to selected source/model/language and appearance; models, recordings, sessions, transcript, capture and export remain core-owned.
- The project can optionally build and copy `rimv_core_ffi.dll` using `BuildRimvCore=true`; the default build expects the matching DLL beside the app. The existing HTML app directory has not been modified.
- The shared FFI now exposes build capability through model state: inference backends absent from the Windows runtime are labeled `unsupported` and cannot be installed or selected. The current Windows feature set compiles Parakeet but not Whisper, so Whisper catalog entries are not presented as usable.

## Feature and usability review

Implemented paths include System/Microphone/Both selection gated by runtime capability; Start/Stop without opening the viewer; language/model selection; real model installation progress; recording search/open/rename/delete; active-session delete prevention; transcript snapshot and ordered update handling; completed recording playback; shared TXT/JSON export through the native save picker; light/dark/system appearance; high contrast; and a per-monitor-aware tray popup.

The usability heuristic pass focused on visible system status, matching core state to UI state, user control over viewer/window dismissal, consistent native navigation, destructive-action confirmation, recovery-oriented errors, and accessible names/focusable WinUI controls. Partial transcript changes are deliberately not announced as a live region on every update to avoid screen-reader chatter. High contrast overrides the selected appearance and open windows respond to changes.

This is a source-level heuristic review, not a real Windows usability session. No screenshots or participant observations are claimed.

## Windows runtime and verification still required

This implementation was authored on macOS (arm64). The development environment has .NET SDK 10.0.105 but no Windows runtime, Windows App SDK runtime, Visual Studio Windows toolchain, WASAPI device, or ability to launch WinUI desktop windows. The `dotnet build apps/windows-native/RimV.Windows.csproj -p:EnableWindowsTargeting=true -p:BuildRimvCore=false` restore reached NuGet and failed with `NU1301` because this environment could not resolve/reach NuGet.org. Therefore no successful WinUI build or Windows launch is claimed.

Checks run on this machine:

- `cargo fmt --all -- --check` — passed.
- `cargo test --locked --offline -p speech-transcription --no-default-features` — passed, 22 tests.
- XML parsing for all 7 XAML files, the project file and app manifest — passed (10 files).
- `git diff --check` — passed.
- `cargo test --workspace --locked --offline` — blocked in existing Apple native dependencies before the workspace suite could complete. `apple-cf` could not write to the sandboxed clang module cache; `screencapturekit` also reports dependency SwiftLint errors in vendored/generated Swift sources. No application behavior was changed to work around those external build failures.
- WinUI `dotnet build` — restore failed with `NU1301` for unreachable `https://api.nuget.org/v3/index.json`; compilation did not begin.

The following are pending on a Windows 11 development machine:

1. Restore/build the project for x64 and ARM64 with Windows App SDK 2.0.1; fix any generated XAML/compiler or API mismatches found there.
2. Launch the unpackaged executable and verify no terminal/browser appears, tray creation/callbacks work, and the popup positions/dismisses correctly at taskbar edges and multiple DPI values.
3. Exercise light/dark/high contrast, keyboard navigation, native window reopen/close, single-instance behavior and graceful shutdown.
4. Build `rimv-ffi` for Windows and place the resulting DLL beside the app; validate FFI ABI version and every JSON request/response against the native Rust binary.
5. Exercise real microphone, system loopback and Both capture; verify Start/Stop, live transcript snapshot/event ordering, persisted sessions across restart, model loading, errors, playback, rename/delete and both exports.
6. Run the Rust workspace regression suite on supported platforms and Windows-specific CI. macOS must retain its existing build/test baseline.

## Current runtime limitations and risks

- The Windows shared-core feature matrix currently excludes Whisper; the UI now communicates this rather than deferring failure until model activation.
- Physical audio routing, microphone permissions, loopback capture and playback are unverified until real Windows hardware/runtime testing.
- WinUI XAML generation, App SDK deployment prerequisites, Shell notification callback behavior, tray icon resource loading and DPI placement require Windows validation.
- Model downloads and inference run through shared-core calls and event polling; responsiveness and cancellation under real model sizes need runtime verification.
- The unpackaged project is intentionally not an installer. A matching `rimv_core_ffi.dll` must be built and shipped beside the executable.

## Exit assessment

The native shell and Phase 1 integration boundary are implemented in source, with explicit UI states for missing runtime capability. Phase 2 acceptance is **not signed off** until the Windows build/launch and physical-device scenarios above pass. Installer and release work remain out of scope.
