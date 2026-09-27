# rimv Releases

## v0.1.0-beta

First public beta preparation for rimv, Real-time Intelligent Multilingual
Voice.

### Highlights

- Local Rust audio engine for microphone and platform system-audio capture.
- Independent microphone and system-audio sources.
- Recording-first runtime: audio recording survives VAD, ASR, and model failures.
- Bounded realtime queues between capture, recording, and transcription work.
- Live CLI partial transcript rendering without duplicating cumulative hypotheses
  as permanent terminal lines.
- Desktop interfaces on macOS and Windows, with platform-specific layouts and
  shared local capture and recording workflows.
- Main `rimv` developer CLI with listen, transcribe, models, devices, doctor,
  serve, and benchmark commands.
- Experimental local web/server path for development preview.

### Desktop UI

| Platform | Interface in this beta |
|---|---|
| macOS | Native menu-bar popover for capture status and source selection, with native model/language selectors and recording/transcript windows. |
| Windows | Native WinUI 3 desktop application with a notification-area menu, independent language/model/recordings/transcription windows, and native settings. It uses the shared Rust core for model management, sessions, transcripts, and exports. The existing HTML/Tauri Windows app remains a separate implementation. |

The native Windows installer has passed automated clean-install, first-launch,
reinstall, uninstall, and user-data-preservation checks on GitHub Actions.
Interactive tray behavior, accessibility/scaling, and physical microphone and
WASAPI capture still need validation on Windows hardware. macOS remains the
reference UI for feature behavior and visual review.

### Distributions

| Distribution | Artifact intent | Contents |
|---|---|---|
| rimv Capture | Lightweight capture-only package | `rimv-capture`, `LICENSE`, distribution metadata |
| rimv Transcribe | Capture plus local transcription runtime | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| rimv Server | Experimental localhost API package | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| RimV Desktop for Windows | Per-user Windows desktop app installer | Native WinUI 3/.NET app, shared Rust core and native ASR runtime DLLs; model weights remain separate |
| rimv Full | Ready-to-use transcription package with bundled model weights | Omitted from this beta |

The macOS menu-bar application is distributed separately from these CLI
archives. The Windows desktop interface is delivered as the per-user installer
listed above.

rimv Full is omitted because exact redistribution terms and attribution for the
third-party model weights have not been verified. Model weights are installed or
configured separately.

### Platform Status

| Platform | Status |
|---|---|
| macOS arm64 | Primary beta target. Workspace checks, staged Capture/Transcribe/Server outputs, and previous local microphone/system/transcription runs were exercised on this host. |
| Windows x64 | Preview release. The release workflow produces CLI archives and a per-user NSIS installer for the native WinUI 3 app. Automated install, startup, reinstall, uninstall, and user-data-preservation checks pass; interactive UI and hardware audio validation remain pending. Installers are signed only when protected release signing credentials are configured. |
| Linux x64 | Experimental/partial. Microphone capture uses CPAL/ALSA. System audio capture is unsupported pending a PipeWire backend; no public beta binary is published. |

Compilation does not imply runtime support. Capture permissions, audio devices,
and model runtime behavior must be validated on real target machines.

### Known Limitations

- Transcription uses VAD-segmented offline recognition over finalized speech
  segments; it is not true streaming ASR.
- Model weights are not bundled in beta artifacts.
- Linux system audio capture is not implemented.
- Windows tray/accessibility behavior and microphone/WASAPI capture need runtime validation on Windows hardware.
- The native Windows installer is unsigned unless signing credentials are configured. Uninstall preserves `%LOCALAPPDATA%\RimV` user data.
- The local server and web UI are experimental.
- macOS community builds are unsigned or ad-hoc signed and not notarized.

### Benchmark Note

One internal pre-beta `realtime_capture` benchmark case measured:

| Metric | Result |
|---|---:|
| WER | 0.112 |
| CER | 0.127 |
| RTF | 0.028 |
| Cases | 1 |

This is a single internal measurement, not a universal accuracy or performance
claim.

### Compatibility And Upgrade Notes

- Product and package metadata are aligned to `0.1.0-beta`.
- Development model files remain outside Git under `resources/models/`.
- Existing local recordings, benchmarks, and generated target artifacts are not
  part of release packages.
- Future signed macOS releases may add Developer ID signing and notarization, but
  the free beta flow does not require Apple credentials.
