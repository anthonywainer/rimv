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
- Native macOS menu-bar app for local capture control.
- Main `rimv` developer CLI with listen, transcribe, models, devices, doctor,
  serve, and benchmark commands.
- Experimental local web/server path for development preview.

### Distributions

| Distribution | Artifact intent | Contents |
|---|---|---|
| rimv Capture | Lightweight capture-only package | `rimv-capture`, `LICENSE`, distribution metadata |
| rimv Transcribe | Capture plus local transcription runtime | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| rimv Server | Experimental localhost API package | `rimv`, native ASR runtime libraries, `LICENSE`, distribution metadata |
| rimv Full | Ready-to-use transcription package with bundled model weights | Omitted from this beta |

rimv Full is omitted because exact redistribution terms and attribution for the
third-party model weights have not been verified. Model weights are installed or
configured separately.

### Platform Status

| Platform | Status |
|---|---|
| macOS arm64 | Primary beta target. Workspace checks, staged Capture/Transcribe/Server outputs, and previous local microphone/system/transcription runs were exercised on this host. |
| Windows x64 | Preview. Code contains microphone and WASAPI loopback capture paths, but this beta has not been runtime-tested on Windows hardware. |
| Linux x64 | Experimental/partial. Microphone capture uses CPAL/ALSA. System audio capture is unsupported pending a PipeWire backend. |

Compilation does not imply runtime support. Capture permissions, audio devices,
and model runtime behavior must be validated on real target machines.

### Known Limitations

- Transcription uses VAD-segmented offline recognition over finalized speech
  segments; it is not true streaming ASR.
- Model weights are not bundled in beta artifacts.
- Linux system audio capture is not implemented.
- Windows capture and transcription need runtime validation on Windows hardware.
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
