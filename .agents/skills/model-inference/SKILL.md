---
name: model-inference
description: "Use for ONNX/Whisper model loading, cache paths, model downloads, integrity, CPU/GPU selection and inference lifecycle in RimV. Excludes unrelated web UI changes."
---

# Local Model Inference and Lifecycle

## Goal
Make local speech model installation and inference reliable, observable and resource-aware.

## Map the model lifecycle
```text
not installed → downloading → verifying → installed
             → loading → ready → in use → unloading
                         ↘ failed/retry
```
Identify which crate owns each transition (`model-manager`, `whisper-models`, engine integration); do not let frontend or platform adapters silently become the source of truth.

## Procedure
1. Determine the exact model ID/version/backend and supported platforms from project configuration.
2. Identify required files, expected checksums/signatures if available, local storage path and disk-space needs.
3. Check download resume/atomic rename behavior: incomplete files must not appear ready.
4. Confirm model/backend compatibility and provide actionable error states for unsupported hardware or missing libraries.
5. Make initialization and warm-up observable; separate installed from loaded and loaded from ready.
6. Define cancellation, retry, unload and switch-model behavior. Release old resources on failure.
7. Measure cold load, peak memory and inference latency on representative hardware before changing defaults.

## Resource rules
- Do not load duplicate model instances unintentionally.
- Avoid unbounded in-memory audio batches.
- Verify native library availability and architecture (`arm64`, `x64`, etc.) rather than assuming macOS-only paths.
- Keep runtime backend selection explicit and log diagnostic codes without exposing private audio or paths unnecessarily.
- Use a safe rollback if an update fails verification.

## Testing
Unit-test manifest/state transitions and corrupt/incomplete download handling without network. Integration-test one pinned model on available hardware; document what platform/backend was actually tested. Do not claim universal hardware support from one machine.

## Related
`security-privacy` for downloads and file integrity; `performance-profiling` for measured optimization; `cross-platform` for runtime binary distribution.
