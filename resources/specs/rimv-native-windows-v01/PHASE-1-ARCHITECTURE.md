# RimV Native Windows v0.1 — Phase 1: Shared Rust Core Integration

**Status:** Implemented in the Rust workspace; Windows runtime validation remains pending.

## Outcome

The existing `engine-runtime` and `model-manager` crates are the authoritative shared implementation for recording sessions, live transcript state, persisted recording metadata, transcript exports, model catalog/install state and model selection validation. Both the existing macOS host and existing Tauri Windows host now use those shared services for the migrated operations. A new `rimv-ffi` `cdylib` exposes those services to a future WinUI 3/C# host through a versioned C ABI carrying UTF-8 JSON requests and responses.

No WinUI application was created in this phase. The working macOS UI and current HTML/Tauri Windows application remain separate, preserved hosts. Native menus, windows, file save dialogs and audio capture remain host responsibilities.

## Workspace and ownership

| Responsibility | Canonical implementation | Host/platform boundary |
|---|---|---|
| Commands, capture lifecycle and runtime state | `crates/engine-runtime` + `crates/engine-protocol` | Host supplies platform audio backend and issues commands |
| Transcript update aggregation for live viewer snapshots | `engine-runtime::EventBus` | Hosts subscribe/poll and render updates |
| Persisted session listing, detail, title sidecar, safe deletion and TXT/JSON serialization | `engine-runtime::RecordingLibrary` | Hosts choose dialogs/destination and display DTOs |
| Model catalog, managed installation, readiness, runtime path and supported languages | `crates/model-manager` | Hosts render progress and persist UI preferences |
| macOS menu/windows, permissions, ScreenCaptureKit and native file panels | `apps/menu-bar` and `apps/menu-bar/native` | macOS only |
| Windows Tauri webview, commands, permissions, WASAPI/CPAL and audio playback endpoint | `apps/windows` | Existing HTML implementation, retained |
| Future native Windows UI | Not created | WinUI 3/C# adapter will call `rimv-ffi`; it must contain no ASR/model/session/export business logic |

Other shared workspace crates remain available to both hosts: `audio-core`, `audio-capture`, `speech-transcription`, `app-paths`, `system-profile` and `whisper-models`. The audio capture drivers and native desktop code are deliberately platform-specific.

## Consolidated duplication

- Removed app-local live transcript caches from macOS and the existing Windows host. The runtime event bus now retains the current utterance updates and exposes `LiveTranscriptSnapshot`, resetting it when the active session changes.
- Moved recording enumeration, metadata identity checks, titles, detail loading, rename/delete rules and transcript export formatting into `RecordingLibrary`. macOS and Tauri call this service rather than independently interpreting session JSON or formatting export data.
- Centralized model inference path resolution and model language validation in `ModelManager`; both current hosts use these shared methods.
- The HTML Windows viewer still renders transcript rows, and macOS still displays transcripts through AppKit. This presentation work is intentionally host-specific; neither owns transcript state or export serialization.

The UI adapters continue translating shared DTOs to existing UI shapes. macOS preserves its native localized default recording label when a session has no user title.

## Native Windows FFI contract

Sources: `crates/rimv-ffi/include/rimv_core.h` and `crates/rimv-ffi/src/lib.rs`.

- ABI version: `rimv_api_version()` currently returns `1`.
- Lifecycle: `rimv_engine_create(config_json, &handle)`, `rimv_engine_request(handle, request_json)`, `rimv_string_free(result)`, `rimv_engine_destroy(handle)`.
- Configuration: JSON requires `recordings_directory`; `models_directory` is optional.
- Requests are tagged JSON objects. The initial API includes runtime state/commands, event polling, model catalog/install/cancel/select/language/clear, recording list/detail/rename/delete/export, and shutdown. See the Rust `Request` enum as the contract source.
- Responses are JSON envelopes: `{"ok":true,"result":...}` or `{"ok":false,"error":{"code":"request_failed","message":"..."}}`. Error strings are user-safe operation context, not a stable diagnostic code taxonomy yet.
- Every returned string is UTF-8, allocated by Rust and freed exactly once with `rimv_string_free`. Handles are opaque and must be created/destroyed once. The caller must not use a handle concurrently with destroy.
- No Rust panic is intended to cross create/request. Inputs must be valid NUL-terminated UTF-8 pointers. The C# adapter must marshal them accordingly and release every returned string in `finally`/`SafeHandle`-style ownership.
- `poll_event` is bounded to 30 seconds and must run away from the UI dispatcher. It returns at most one runtime event or model progress item per call. The C# adapter should serialize commands/lifecycle and poll from a background task.
- Model installation runs on Rust worker threads and can be cancelled. Progress is sent through a bounded queue; under a sustained queue backlog intermediate progress may be dropped. Completion/error progress is best effort in this initial contract.
- `Export` returns serialized transcript content. The native UI owns the save dialog and destination write. Recording paths are resolved inside the configured recordings root in shared Rust code; callers pass IDs, not arbitrary paths.

The FFI is an integration boundary, not a second engine. The future C# layer should be limited to DTO conversion, async polling/cancellation, lifecycle ownership and WinUI-specific presentation/permissions. Before shipping, add a checked-in C# binding and validate ABI loading, architecture mapping, string ownership and lifecycle on Windows.

## Platform risks and limits

- Windows build and runtime execution could not be validated on this macOS development machine. WinUI loading, C# marshaling, MSVC linking and x64/ARM64 DLL packaging remain pending Windows CI/hardware validation.
- Existing Windows ASR native dependency support remains the major parity risk: build availability of whisper-rs-sys/Whisper and bundled sherpa-onnx runtime assets must be confirmed for the target Windows architectures. This phase does not add or duplicate an ASR engine.
- macOS menu/bar code still owns ScreenCaptureKit, AppKit window state, microphone/system-audio permissions, and presentation. The existing Windows host still owns WASAPI/CPAL integration, Tauri windows and browser rendering.
- Rust shared core changes apply to both current Rust hosts after rebuilding. Native WinUI will receive them only after rebuilding and distributing the matching `rimv_core_ffi` library.
- The current native macOS behavior is preserved by projecting shared data into existing UI structures. The old macOS fallback title formatting is intentionally retained.

## Validation

- `cargo metadata --no-deps --format-version 1 --offline`: passed (workspace manifests resolve without fetching dependencies).
- Direct stable `rustfmt --edition 2024 --check` on all modified Rust source files: passed.
- `cargo test -p engine-protocol -p model-manager --locked`: passed; all 5 model-manager tests passed (engine-protocol has no unit tests).
- `make test`: attempted for the full workspace, but stopped in the existing `apple-cf` and `screencapturekit` build scripts. SwiftPM could not write its Clang module cache under the sandbox, and the dependency's SwiftLint run also reported errors. The runtime/recording-library/FFI test binaries could not be built on this machine.
- `node --check apps/windows/ui/main.js` and `git diff --check`: passed.
- Windows runtime/FFI tests: pending a Windows runner. macOS application build/runtime regression testing is also pending dependency resolution on this machine.

## Next phases and acceptance gates

1. **WinUI shell and C# adapter:** create a separate native Windows project and SafeHandle-based binding without changing `apps/windows`; acceptance requires loading the built DLL, creating/shutting down the engine, and polling on a worker thread on x64 Windows.
2. **Windows capture and permissions:** provide WASAPI capture as an `engine-runtime` backend; acceptance requires explicit microphone/system-loopback permission/error UX and lifecycle tests while preserving shared transcription.
3. **Native parity UI:** implement settings, model, recording and viewer screens with shared FFI DTOs; acceptance requires no copied model/session/export logic and parity with existing workflows.
4. **Windows inference/package parity:** validate Whisper/sherpa native dependencies per supported architecture and package required runtime assets; acceptance requires offline inference and model lifecycle checks on clean Windows machines.
5. **Cross-platform regression/release readiness:** run macOS and Windows build/test matrices, ABI compatibility checks and behavior comparisons; do not delete the Tauri/HTML implementation until a separately approved migration decision.

## Approval boundary

This phase follows the approved Phase 0 direction: one shared Rust core, opaque versioned C ABI, thin future C# adapter, and preservation of the Tauri HTML app. No remaining architecture choice blocks Phase 1. Before adding a WinUI project, confirm whether it should become the default Windows product while retaining the Tauri app as a fallback, and select the supported Windows architectures for the native dependency/package matrix.
