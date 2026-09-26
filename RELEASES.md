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
| Windows | Tauri desktop window with Capture, Models, Recordings, and Settings sections, plus a notification-area menu. Capture offers microphone, system audio, or both; recordings include transcript viewing, audio playback when capture has stopped, rename/delete, and TXT/JSON export. |

The Windows interface is a preview and still needs runtime validation on Windows
hardware. macOS remains the reference UI for feature behavior and visual review.

### Distributions

| Distribution | Artifact intent | Contents |
|---|---|---|
| rimv Capture | Lightweight capture-only package | `rimv-capture`, `LICENSE`, distribution metadata |
| rimv Transcribe | Capture plus local transcription runtime | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| rimv Server | Experimental localhost API package | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| RimV Desktop for Windows | Per-user Windows desktop app installer | Tauri app, WebView2 bootstrapper, Sherpa/ONNX runtime DLLs; model weights remain separate |
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
| Windows x64 | Preview release. The active workflow currently builds and uploads only the per-user Tauri installer; full release validation, CLI archives, checksums, and GitHub Release publishing are paused. The application has not been runtime-tested on Windows hardware. |
| Linux x64 | Experimental/partial. Microphone capture uses CPAL/ALSA. System audio capture is unsupported pending a PipeWire backend; no public beta binary is published. |

Compilation does not imply runtime support. Capture permissions, audio devices,
and model runtime behavior must be validated on real target machines.

### Known Limitations

- Transcription uses VAD-segmented offline recognition over finalized speech
  segments; it is not true streaming ASR.
- Model weights are not bundled in beta artifacts.
- Linux system audio capture is not implemented.
- Windows capture and transcription need runtime validation on Windows hardware.
- The Windows installer downloads WebView2 Bootstrapper when needed; first install then requires internet access. Uninstall preserves `%LOCALAPPDATA%\RimV` user data.
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
