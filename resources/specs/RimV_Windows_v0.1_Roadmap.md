# RimV Windows v0.1 — Development Specifications

**Status:** Phases 1–6 implementation changes are in the working tree; Windows hardware/runtime validation and release gates remain open
**macOS:** Working feature baseline; freeze product features until Windows parity
**Windows UI decision:** Tauri desktop shell with in-process typed Rust commands/events and a native tray
**Visual references:** `resources/specs/template/` (nonfunctional preview only)

## Repository findings and Phase 0 results

### Existing architecture

```text
apps/menu-bar (macOS AppKit + Rust controller)
        │ direct typed EngineRuntime commands/events
        ├── EngineRuntime ── Engine protocol / snapshots / session persistence
        │       ├── audio-capture ── CPAL mic; ScreenCaptureKit system audio on macOS
        │       └── speech-transcription ── Parakeet/sherpa-onnx; optional Whisper/whisper.cpp
        └── model-manager (catalog, downloads, verification)

apps/web ── experimental Vite client ── localhost WebSocket (127.0.0.1:7878)
```

- Portable Rust domain/runtime: `audio-core`, `engine-protocol`, `engine-runtime` orchestration/storage, `speech-transcription`, `model-manager`, `whisper-models`, and most of `system-profile`. `engine-protocol` is UI independent and serializable. `EngineRuntime` already offers typed `send`, `snapshot`, `subscribe`, and `shutdown` operations.
- Audio boundary: `audio-capture` uses CPAL for microphone capture. `platform/macos.rs` uses ScreenCaptureKit for system audio. `platform/windows.rs` currently routes system capture through CPAL output-device loopback (WASAPI through CPAL); it is not yet Windows-verified. Linux system capture explicitly returns unsupported.
- macOS-only frontend: `apps/menu-bar/src/macos.rs` and `apps/menu-bar/native/*.m` implement the app controller, AppKit status item, selector popovers, model manager, recordings selector, viewer and media player. `build.rs` compiles Objective-C with `xcrun clang` and links AppKit, Foundation and AVFoundation. The package manifest only enables runtime dependencies on macOS, and `main.rs` reports unsupported OS elsewhere.
- Current experimental web client: `apps/web` is plain TypeScript + Vite, with a WebSocket client; it has no Tauri dependency. Its TypeScript protocol currently omits several Rust commands and snapshot fields, so it is not yet a complete authoritative contract. The loopback WebSocket bridge binds port 7878 without authentication and is an experimental local bridge, not the recommended desktop IPC boundary.
- ASR: Parakeet TDT 0.6B v3 INT8 and Silero VAD use sherpa-onnx 1.13.8 shared native libraries. Whisper is optional at crate level but enabled by `engine-runtime`; `whisper-rs-sys` builds vendored whisper.cpp/ggml native C/C++ code. Metal/CoreML are macOS-specific accelerators; the vendored build script has Windows CUDA/OpenBLAS/Vulkan paths, but default Windows CPU compilation still needs a real MSVC/clang-cl validation. Windows ARM64 is particularly risky: the repository patch comment explicitly calls out the need for clang-cl rather than MSVC for GGML.
- `model-manager` owns a catalog, download, archive extraction, SHA-256 verification, install-state checks and removal. Its default model path is hardcoded under `HOME/Library/Application Support/rimv/models`, which is a portability defect to address in Phase 1. The macOS app separately saves preferences under `~/Library/Application Support/rimv`.
- Sessions are directories under the configured recordings directory, named by stable UUID. `engine-runtime/src/storage.rs` writes per-source float WAVs, schema-versioned `session.json`, and finalized `transcript.json` / `transcript.txt`; partial transcript updates are volatile. The macOS shell adds a `rimv-session.json` sidecar for friendly title metadata. It enumerates recordings, validates paths, renames via sidecar, prevents deletion of the active session, and exports completed transcript files through native save panels. Keep UUIDs and existing session layout compatible.

### Template and current macOS UI comparison

The six HTML files are `index.html`, `menubar.html`, `language.html`, `models.html`, `recordings.html`, and `viewer.html`; shared assets are `style.css` and `preview.js`.

- `index.html` is an overview/documentation page, not a product screen.
- `menubar.html` depicts a 380px tray popover, source selection, start/stop, language/model/recordings entry points and quit. Current AppKit UI has those functions, but actual layout, native menus, status item, focus and window behavior come from Objective-C and vary from the HTML approximation.
- `language.html` has searchable/popular/all languages. `preview.js` hardcodes a sample list. The real app derives supported languages from the selected model and sends `SetTranscriptionLanguage` through the runtime.
- `models.html` represents the selector and manager tabs. The preview seeds Parakeet as installed, shows mock Whisper data and simulates downloads/progress. Real catalog/state/downloads are supplied by `model-manager` and published to AppKit; do not infer installed status or available actions from the mock.
- `recordings.html` preview seeds fictional sessions; rename/delete persist in browser `localStorage`. Actual recordings are discovered from on-disk `session.json` plus the active snapshot; names are stored in `rimv-session.json`. Search/filter/open/rename/delete are real AppKit controller actions.
- `viewer.html` includes fictional Spanish transcript/audio, decorative waveform, simulated playback and export placeholders. Current AppKit viewer displays actual partial/final transcript state, loads saved transcript/audio, plays files with AVFoundation and exports TXT/JSON via save panels. Viewer visibility is independent of engine capture.
- `style.css` defines a teal accent and simulated desktop styling. It is a visual reconstruction, not a source of approved product tokens or exact native geometry. Preserve functional macOS behavior; adapt Windows visuals to Windows conventions.
- `preview.js` explicitly says it makes no real recordings, transcriptions or downloads. Its global mock model/session state is initialized from seed data and browser localStorage. It is useful only for visual/layout review.

### Frontend decision: Tauri

Use Tauri 2 for a Windows desktop/webview shell around the existing TypeScript frontend, with Windows tray/window integrations exposed as narrow Rust commands/events. Keep one process-owned `EngineRuntime`; commands call its typed API and events subscribe to its single event stream. The UI must not instantiate ASR or treat local storage as engine state. Reuse engine protocol types where practical, or define a versioned app command/event DTO at the shell edge.

Tradeoffs: Tauri reuses the existing Vite/TypeScript work and Rust ownership model and avoids a separately deployed web server. It still requires a WebView2 runtime and explicit window/tray lifecycle work; HTML/CSS does not make the tray native automatically. A direct in-process binding keeps typed ownership but needs a carefully managed async/event bridge. Do not expose the current unauthenticated port 7878 bridge to a packaged UI as the primary boundary. Native WinUI/Win32 would offer tighter Windows conventions and less web runtime surface, but would duplicate frontend work and require a new Rust interop boundary. The Phase 1 spike must verify Windows tray, hidden/background window lifecycle, event delivery, packaging and WebView2 assumptions before commitment becomes irreversible.

### Smallest safe shared/platform interface

1. Keep `EngineRuntime`, `EngineCommand`, `EngineEvent`, snapshots, transcript update types, model catalog/state and session formats platform-neutral.
2. Define audio capture at the existing `AudioCapture` + `FrameReceiver` interface: `start`, `stop`, `is_running`, metrics, and timestamped `AudioFrame`s. Implement microphone/system adapters behind `cfg(target_os=...)`; UI code never calls CPAL, WASAPI or ScreenCaptureKit.
3. Desktop-shell boundary: commands for engine capture/configuration and model/session operations; a subscription carrying authoritative snapshots, transcript updates and errors; explicit shutdown. Keep UI window show/hide/close and tray behavior in the host adapter. Avoid adding a broad abstraction until Windows has a second real consumer.
4. Move per-user paths behind a small platform-independent path/config provider or inject resolved model/preferences/recording roots from each shell. Preserve file formats and identifiers.
5. Model downloads and session listing/name/delete are currently largely shell orchestration, not complete `EngineRuntime` APIs. Phase 1 should expose them through a small app service or command layer without pushing UI concerns into audio/transcription crates.

### Windows build risks

- No Windows target build was run in this macOS-only phase. A Windows host or configured cross-compilation toolchain is required to verify actual linkage.
- `apps/menu-bar/build.rs` is intentionally macOS-specific; Cargo manifest and app entry point currently have no Windows GUI implementation.
- `audio-capture/src/platform/windows.rs` has an initial CPAL/WASAPI loopback implementation but is unverified for endpoint selection, permission/error mapping, device changes, and capture stability.
- Sherpa-onnx is dynamically linked; the correct Windows x64/ARM64 binaries, dependent ONNX Runtime DLLs, loader paths and packaging must be verified. Never ship macOS dylibs.
- whisper.cpp CMake/bindgen toolchain requirements include CMake, a working libclang/bindgen setup, C/C++ compiler and Windows linker. MSVC vs clang-cl, static CRT/runtime choice, architecture and CPU feature flags are unresolved; ARM64 is explicitly called out in the vendor patch.
- Platform storage paths assume macOS `HOME/Library/...` in `engine-runtime` and `model-manager`. Use Windows known folders/AppData for user data and keep recordings user-selectable/configurable.
- Tauri adds WebView2, installer, signing, tray and multi-window lifecycle concerns. Confirm exact Tauri version, Windows support and packaging choices during Phase 1.

### Baseline validation (macOS arm64, 2026-09-26)

- `cargo check --workspace --locked` — passed using installed Rust 1.98.1 toolchain explicitly.
- `cargo test --workspace --locked` — failed compiling a test in `apps/rimv-cli/src/terminal_output.rs`: `Instant` is used but not imported (two E0433 errors). No tests executed to completion.
- `make check` — stops at `cargo fmt --all --check`, reporting pre-existing formatting diffs across several files; it therefore does not progress to lint/tests.
- `cargo build --locked -p rimv-menu-bar` — passed.
- `rimv-menu-bar --self-test` — direct launch first needed the sherpa library path; with `DYLD_LIBRARY_PATH=target/debug`, it returned `Operation not permitted` in this environment, so native UI self-test was not verified. `make menu-test` was not completed because it requires app packaging/signing/opening through the Make target.
- No source behavior was changed during validation. Build artifacts are ignored under `target/`.

## Non-negotiable rules

1. macOS remains the feature baseline and receives no product feature changes until Windows parity; regression fixes remain allowed.
2. Reuse shared Rust engine logic where portable. Isolate native audio, tray and window code by platform.
3. Start/stop, capture and viewer visibility have independent lifecycles. Starting must not open the viewer; closing a viewer must not stop capture or quit RimV.
4. Treat the HTML pack as reference only. Backend snapshots, model/session services and persisted files are authoritative.
5. Complete and review one phase before advancing. Compilation alone is not parity; validate on Windows hardware and rerun macOS regressions after shared changes.

## Phase 0 — Architecture and baseline (complete)

**Objective:** Document actual architecture, UI references, boundaries, risks and macOS validation without changing application behavior.

**Exit gate:** Findings reviewed and Phase 1 explicitly approved before implementation.

## Phase 1 — Shared Rust boundary and Windows build proof

**Objective:** Make a minimal, behavior-preserving core integration plan compile on Windows.

**Scope:** Resolve platform-specific app data/model paths through an injected path provider; ensure app crate target dependencies and entry points are explicit; define the shell command/event service over `EngineRuntime`; provide a Windows x64 build proof for shared runtime and selected ASR libraries; spike Tauri 2 tray/window lifecycle and typed event delivery. Do not change macOS capture/transcription semantics.

**Deliverables:** Reviewed command/event mapping; verified Windows x64 core build and native DLL inventory; Tauri shell proof covering tray, background runtime and window lifecycle; exact macOS regression baseline.

**Completed slice (2026-09-26):** Added `crates/app-paths` as the shared per-user data-path resolver. macOS keeps `~/Library/Application Support/rimv`; Windows uses `%LOCALAPPDATA%/RimV` (falling back to `<USERPROFILE>/AppData/Local/RimV`); Linux uses `$XDG_DATA_HOME/rimv` (falling back to `~/.local/share/rimv`). ModelManager, EngineConfig and the legacy whisper-models path now share this resolver; the CLI also falls back from `HOME` to `USERPROFILE`. `RIMV_MODELS_DIR` remains authoritative. This changes only default model discovery paths off macOS and preserves the macOS path.

**Validation so far:** Focused macOS checks for `app-paths`, `model-manager`, `whisper-models`, `engine-runtime` and `rimv` pass. Unit tests for `app-paths`, `model-manager`, and `whisper-models` pass (7 tests total). `app-paths` also cross-checks for `x86_64-pc-windows-gnu`. A broader GNU target check failed in dependency `ring` before RimV compilation because MinGW is absent. No native Windows/MSVC build, native DLL inventory, Tauri tray/window proof, or Windows runtime verification has been completed.

**Remaining Phase 1 work:** Run the existing Windows x64 MSVC CI build; inspect/stage and validate ASR DLL dependencies; create the minimal Tauri shell proof for one engine owner, typed command/event delivery, tray and hidden-window lifecycle; record macOS regression results after shared changes. Do not advance the phase exit gate until these checks pass.

**Exit gate:** Windows core links with a selected inference backend and the shell proof can own one engine instance; unresolved native dependency choices are recorded.

## Phase 2 — Windows audio and CLI validation

**Objective:** Verify microphone, output loopback and combined capture through the current frame interface before building product UI.

**Scope:** Exercise and, only where evidence requires, repair `audio-capture` Windows adapter; validate device enumeration, formats, clocks, errors and stop/finalization; run real CLI capture/transcription and check WAV/TXT/JSON artifacts using Windows-compatible ASR runtime.

**Deliverables:** Windows CLI capture report with test machine, endpoints, model binaries, real saved artifacts and device-loss/long-session observations.

**Exit gate:** Microphone, system, and both modes work on a Windows target with stable saved output.

## Phase 3 — Windows desktop shell and tray

**Objective:** Wire real engine state to the chosen Windows desktop UI.

**Scope:** Tray icon/popover, source selector, status, start/stop, settings/model/recordings navigation, quit, themes and persistent process lifecycle. One owner creates the runtime; window close does not stop it. Translate commands into typed Rust APIs and render snapshots/errors from subscriptions.

**Deliverables:** Windows desktop shell controlling genuine capture without viewer auto-open.

**Exit gate:** Start/stop and tray lifecycle pass on Windows; capture remains active while secondary windows close.

### Implementation update (2026-09-26)

**Status:** A Tauri 2 shell prototype is in `apps/windows`. It is not yet a Phase 3 exit-gate pass: Windows UI, tray behavior, capture lifecycle, and target linking require validation on a Windows host.

- The shell creates exactly one `EngineRuntime`, exposes typed `get_state`, `set_capture_source`, `set_listening`, and `quit_app` Tauri commands, and forwards the runtime subscription over the `engine-event` event. Capture changes and start/stop run via `spawn_blocking`; a shared command gate serializes UI and tray transitions. Closing the main window hides it while the tray process retains engine ownership. Tray menu actions open, toggle capture, and quit.
- The static, dependency-free frontend displays actual `EngineSnapshot` source/status/timing/transcription availability. Capture is functional through shared runtime APIs. Models, recordings, and settings navigation are explicitly marked as later-phase connections; no fixture/mock records are displayed. Theme follows the OS color scheme, and the shell uses a Windows system-font stack and keyboard focus styles.
- Capture-source choice is stored under the shared per-user app-data root. A process lock prevents a second engine owner. Runtime shutdown is tied to app exit.
- Transcription is enabled only when the configured local model path exists as a file or directory. The shell does not invent or display model catalog/install state.
- A minimal branded PNG supplies window/tray icon pixels. Installer bundling is disabled; package format, signing, WebView2 install mode, and native ASR DLL staging remain Phase 6 decisions.
- The frontend contract relies on the existing serialized `EngineSnapshot` and engine event schema in `engine-protocol`; no shared IPC schema change was introduced. Commands return snapshots/errors, while subscriptions carry actual runtime events.

**Validation:** `node --check apps/windows/ui/main.js`, Rust formatting, workspace metadata, and `git diff --check` passed earlier in this implementation. A Windows MSVC cross-target `cargo check` could not be rerun in this environment: crates.io DNS is unavailable and the local Cargo cache/index is incomplete (`thiserror` is not present for offline resolution). Earlier attempts with the dependencies available reached RimV compilation only after `DOCS_RS=1` bypassed native Sherpa downloads, but Whisper's bundled C/C++ bindings could not cross-build from macOS. Whisper is now excluded from Windows engine-runtime dependencies while remaining enabled for non-Windows targets; Windows selects the existing Parakeet/Sherpa path. This arrangement still needs a real MSVC build and the Windows-native library/DLL verification from Phases 1 and 6.

**Open issues before the exit gate:**

- Run the desktop on supported Windows x64 with the intended Parakeet/Sherpa libraries and verify model availability, mic/system/both switching, saved session output, single-instance behavior, tray actions, hide/reopen, and quit while recording.
- Validate WebView2 availability/installer strategy, Windows high-contrast and scaling, Narrator labels/focus order, and actual app icon rendering.
- Models/recordings/settings screens are navigation stubs. Wire them only in their planned phases after their authoritative model/session services exist.
- `EngineRuntime` errors from tray-start currently surface as an in-app `desktop-error` event; test how that message is presented when the window is hidden and ensure the user can recover.

## Phase 4 — Language and model management

**Objective:** Match real macOS model and language selection semantics.

**Scope:** Derive available languages from selected model metadata; expose actual catalog/install state/download/removal/loading progress; verify artifacts and disk errors; persist selection with the shell’s settings provider.

**Deliverables:** Functional model selector/manager and capability-correct language selector.

**Exit gate:** A verified installed Windows model can be selected and used; displayed languages match its backend capabilities.

### Implementation update (2026-09-26)

**Status:** Windows shell now has a model catalog/selector, Parakeet install progress and cancellation, removal, language selection, and persisted model/language preferences. Phase 4 exit gate remains open until Windows runtime validation confirms download, model loading and transcription.

- The shell reads catalog descriptors and install state from `model-manager`; the frontend receives only the Windows-supported Parakeet backend. Whisper catalog items are omitted because the Windows `engine-runtime` target does not compile Whisper.
- Install uses the existing `ModelManager::install_parakeet` implementation on a named worker thread. It streams progress events, supports cancellation, verifies that the required model files exist after extraction, and cancels/joins tracked workers during app shutdown. Errors and incomplete installs remain visible for retry.
- The shared model-manager now streams async response chunks on its existing worker call path. Connection setup and each stalled body read time out after 30 seconds; the previous 30-second total request deadline would have aborted large model downloads. Cancellation is checked between chunks.
- The UI can cancel during download. Archive extraction is reported as a separate, non-cancellable phase; quitting waits for the active installer worker to finish extraction before shutting down.
- Selection requires a ready model and idle engine. It sets the runtime backend, model path, and validated language using typed `EngineCommand`s. Language choices derive from the selected descriptor; `auto` maps to the runtime’s `None` value. The model and language IDs persist under the per-user app-data root and are restored only when the model is still ready.
- Removing the selected model first clears it from runtime state. The shared engine protocol gained the additive `ClearTranscriptionModel` command so availability and capabilities update immediately rather than retaining a deleted model path.
- The Parakeet catalog currently supplies no archive checksum or expected archive size. Its installer validates that extraction produced all required files but does not establish archive integrity; a signed or checksummed source remains release work. The external archive download and actual ASR DLL/runtime load have not been exercised on Windows.

**Validation:** JavaScript syntax, Rust formatting, Cargo metadata, and `git diff --check` pass. Host macOS `cargo check` passes for `engine-runtime` and `model-manager`; the non-Windows `rimv-windows` entry point and shared protocol also check. MSVC cross-target compilation was attempted, but this macOS host lacks a Windows C/C++ SDK; native dependencies failed compiling at standard headers (`stdlib.h`, `assert.h`) before the shell could be fully checked. Real Windows download, cancellation, removal, language selection and transcription remain unverified.

## Phase 5 — Recordings and transcription viewer

**Objective:** Match real recording discovery, live/completed views and lifecycle.

**Scope:** List sessions from actual files/runtime state; preserve session IDs and schema; rename via metadata sidecar; guard active deletion; live viewer reopens authoritative transcript cache without duplication; completed viewer reads persisted transcript, plays saved audio and exports TXT/JSON. Keep capture independent of window visibility.

**Deliverables:** Functional Windows recordings list and live/completed viewer with playback and exports.

**Exit gate:** Repeated open/close, concurrent capture/playback, rename/delete, restart and export workflows pass on Windows.

**Implementation update (2026-09-26):** Windows now lists active sessions from `EngineRuntime` and completed sessions from validated `session.json` files under the recordings root. Rename titles use a versioned `rimv-session.json` sidecar, while deletion requires a matching persisted session identity and is serialized with capture commands; active/finalizing sessions are protected. The viewer uses the live `TranscriptUpdate` cache keyed by source and utterance ID, and reads completed `transcript.json` records without treating fixture data as state. Saved WAV playback is served through the `rimv-audio` Tauri protocol with session/source allowlists, canonical path checks and byte-range responses bounded to 8 MiB. TXT/JSON exports are created from the selected session transcript in the UI.

**Implementation status:** Code and UI path implemented; Phase 5 exit gate remains open pending Windows runtime validation for playback, repeated viewer opening, rename/delete, restart and exports. Validation on this macOS host: Windows UI JavaScript syntax, Rust formatting and `git diff --check` pass. A temporary host-target compile that enabled the Windows module and Tauri build script passed. A true `x86_64-pc-windows-msvc` check is unavailable here because the target standard library and Windows C/C++ SDK are missing; native audio playback and Windows filesystem replacement behavior still require Windows verification.

## Phase 6 — Packaging and parity release gate

**Objective:** Ship installable Windows v0.1 and verify parity without regressing macOS.

**Scope:** Bundle app, icons, model runtime DLLs and installer; verify WebView2/tray/startup/uninstall/user-data behavior, signing and architecture; run functional matrix on real Windows and macOS machines; document native differences and known limits.

**Deliverables:** Installable package, setup/update/uninstall notes, parity checklist, test results and release notes.

**Exit gate:** Critical workflows pass on real Windows hardware and macOS regression validation passes.

**Implementation update (2026-09-26):** Enabled Tauri's NSIS bundler as the Windows desktop installer. The package uses a per-user install, Tauri's WebView2 download bootstrapper, the Windows icon, and the version from the Cargo workspace. Windows workflows run a preparation script from the repository root to compile the Sherpa runtime and stage its DLLs before Tauri resolves installer resources; the NSIS hook installs them beside the executable and removes them on uninstall. Model weights stay separately installed under `%LOCALAPPDATA%\RimV\models`, while recordings and preferences remain under `%LOCALAPPDATA%\RimV` and outside the application install folder. The unsupported `bundleVCRuntime` setting was removed because the pinned Tauri CLI 2.11.4 rejects it; validating whether native DLL dependencies need separate Visual C++ redistributable packaging remains a Windows release check. The full release workflow is restored, including validation, quality checks, CLI packaging, checksums, and tag publishing. Tag builds import and clean up optional signing secrets `WINDOWS_SIGNING_CERTIFICATE` (base64 PFX) and `WINDOWS_SIGNING_PASSWORD` without logging certificate data; absent credentials produce an explicitly unsigned installer.

**Implementation status:** Packaging and documentation changes are in place. Phase 6 exit gate remains open: this macOS host cannot run or inspect the Windows installer, exercise WebView2 installation/uninstallation or tray startup, validate native DLL loading from the installed app, or run Windows capture/transcription/playback workflows. The release signing path also needs repository certificate secrets configured. Continue to keep Windows at Preview until the real-machine checklist is complete and macOS baseline validation passes.

### Phase 6 verification checklist

- [ ] Build the x64 MSVC app and NSIS installer on `windows-latest`; confirm every runtime DLL listed by `runtime-dlls.txt` is present beside the installed executable.
- [ ] Install on supported Windows x64 systems with WebView2 already present and absent; verify the bootstrapper failure/retry path when the network is unavailable.
- [ ] Launch from Start and the notification area; verify capture continues when the window is hidden, and Quit stops the engine cleanly.
- [ ] Exercise microphone, WASAPI loopback and Both; install/select Parakeet, transcribe speech, then rename/delete recordings, play each saved track, export TXT/JSON and restart the app.
- [ ] Upgrade an existing per-user install and uninstall it; verify the app is removed and `%LOCALAPPDATA%\RimV` user data remains.
- [ ] Verify signed tag artifacts with `Get-AuthenticodeSignature`, or record that the installer is unsigned when signing secrets are absent; check its published SHA-256.
- [ ] Run macOS workspace/build and menu-bar smoke validation against the frozen feature baseline; record host and outcomes before closing the gate.

**Results recorded for this implementation turn:** Windows installer build/runtime, installation, upgrade, uninstall, Authenticode and hardware workflows were not run on this macOS host. CI and release workflow syntax, PowerShell staging scripts, Tauri config parsing and the Windows Rust module's temporary host-target compile passed. The native MSVC target and SDK are not installed locally.

## Phase 2 — Windows audio and CLI validation (implementation update, 2026-09-26)

**Status:** CLI endpoint selection and Windows audio-adapter compile check complete locally; Windows runtime validation remains open.

### Repository inspection

- `crates/audio-capture/src/platform/windows.rs` implements System capture by creating the shared `MicrophoneCapture` over `AudioSourceKind::System`. Its comment records CPAL's WASAPI loopback behavior for render endpoints (`AUDCLNT_STREAMFLAGS_LOOPBACK`). This is an existing implementation path, not proof of endpoint loopback correctness on supported Windows devices.
- Microphone capture shares `MicrophoneCapture::build`, CPAL format negotiation and a bounded `ArrayQueue` callback path. Callbacks normalize supported f32/i16/u16 inputs to interleaved f32 `AudioFrame`s, attach elapsed timestamps, and count drops. Downstream preprocessing handles channel conversion/resampling for ASR. `EngineRuntime` creates independent source workers; Both is therefore two independently clocked capture paths, not a hardware-synchronized pair.
- The `capture-cli` and `rimv`/`transcribe-cli` binaries are in the workspace. Windows release workflows currently check/build the workspace, but they do not exercise real endpoints or assert saved artifacts. README explicitly says Windows is a Preview target pending per-OS capture tests.
- The `rimv listen` CLI already selects System/Microphone/Both, accepts a model path/backend/language, renders partial and final events, drains final events on stop, and stores recordings via the shared runtime. Added `--mic-device` and `--system-device` so IDs shown by `rimv devices` can select input and render endpoints independently, including in Both mode. The engine session format remains unchanged.
- The current “System” adapter accepts the output `device_id`/configuration expected by shared capture, but device enumeration IDs are generated from direction/name/duplicate ordinal, not persistent Windows endpoint IDs. Re-enumeration after device changes is required; default-device changes and active stream recovery are not established by code inspection alone.

### Validation performed

- `cargo check -p audio-capture --locked --offline --target x86_64-pc-windows-gnu` — passed, including the Windows-specific adapter branch. `cargo check -p audio-capture --tests --locked --offline --target x86_64-pc-windows-gnu` — passed. These are cross-compilation checks only, not Windows runtime validation.
- Focused macOS tests with `RIMV_MODELS_DIR=/tmp/rimv-empty-models` and model override variables cleared — audio-capture 5 passed; engine-runtime unit/integration 25 passed; `rimv` 26 passed, 1 ignored; speech-transcription 25 passed, 2 ignored. The first test attempt without isolated model paths failed one existing runtime test because this machine has Parakeet/VAD files installed; rerunning with an empty temporary model root passed.
- `rimv listen --help` confirms both new endpoint selector flags. `git diff --check` passed.
- This host is macOS arm64 and has no Windows endpoints; no real microphone, WASAPI loopback, Both-mode, device-loss, duration, Windows DLL load, or Windows artifact test was possible.
- Attempted `cargo check --workspace --locked --target x86_64-pc-windows-gnu` using the installed Rust target. It failed in transitive `ring` native build before compiling RimV Windows code because `x86_64-w64-mingw32-gcc` is unavailable. This does not indicate a RimV source failure and does not validate the MSVC target used by Windows CI.
- Existing macOS `cargo check --workspace --locked` passed in Phase 0. The workspace test build has the recorded unrelated missing `Instant` import error; it remains an open macOS baseline issue.

### Phase 2 risks and required Windows evidence

1. Run the Windows x64 MSVC workspace check/build on the existing Windows CI runner and retain results for `audio-capture`, `capture-cli`, `engine-runtime`, ASR native dependencies and output DLL staging.
2. On a real Windows machine, enumerate/select a microphone and render endpoint; capture microphone-only, loopback-only, and Both. Confirm System means endpoint loopback and does not silently substitute microphone data.
3. Verify actual formats, channels, timestamps, queue drops and resulting WAV metadata/sample content. Exercise default endpoint selection and explicit device selection.
4. Test denied microphone privacy access, unavailable/removed devices, default endpoint changes, startup failure, immediate stop, repeated start/stop, and clean finalization. Record observed CPAL/HRESULT error mapping.
5. Run a longer session with the selected Windows-compatible Parakeet/sherpa-onnx binaries and verify partial/final transcript behavior plus `session.json`, `transcript.json`, `transcript.txt`, and WAV outputs. Confirm DLLs are loadable from the staged distribution, not only a developer target directory.
6. Test Both-mode timeline behavior with independent source clocks and verify source-preserving transcript and separate WAV tracks; document drift expectations rather than implying synchronized capture.

**Decision:** `platform/windows.rs` already routes output endpoints through CPAL 0.16's WASAPI loopback implementation and microphone/system use the shared bounded PCM callback path; source inspection plus target compilation did not justify rewriting that backend. The CLI gap for explicit device selection is now addressed. Phase 2 exit gate remains open until Windows x64 ASR build/runtime and real capture/save evidence is recorded.
