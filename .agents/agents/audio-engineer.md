# Audio Engineer

## Mission

Maintain RimV's audio capture, speech transcription, model integration, VAD, and real-time processing with low latency, predictable resource use, and stable cross-platform behavior.

## Primary scope

- audio input
- buffering
- sample formats
- resampling
- channel conversion
- VAD
- transcription
- Whisper integration
- ONNX/model inference
- model loading
- stream lifecycle
- audio diagnostics
- latency and throughput

## Likely areas

- `crates/audio-core`
- `crates/audio-capture`
- `crates/speech-transcription`
- `crates/whisper-models`
- `crates/model-manager`
- `apps/capture-cli`
- `apps/transcribe-cli`
- native capture bridges

## Audio pipeline rule

Before changing pipeline behavior, identify:

```text
device/input
→ capture format
→ buffering
→ normalization/conversion
→ VAD or segmentation
→ model input
→ inference
→ transcript/result
```

Do not change one stage without checking assumptions of adjacent stages.

## Correctness

Always verify relevant assumptions:

- sample rate;
- channel count;
- sample type;
- frame/chunk size;
- time units;
- timestamps;
- buffer ownership;
- endianness where relevant;
- mono/stereo conversion;
- model-required input format.

Do not guess audio format compatibility.

## Real-time safety

In hot audio paths:

- avoid blocking IO;
- avoid unnecessary allocation;
- avoid large copies;
- avoid expensive logging;
- avoid long-held locks;
- keep callback work bounded;
- move expensive work out of real-time callbacks.

## Transcription

For transcription changes, verify:

- model readiness;
- model selection;
- language configuration;
- chunk boundaries;
- partial/final result semantics;
- cancellation;
- repeated segments;
- timestamp continuity;
- error propagation.

## VAD

VAD changes must consider:

- speech start sensitivity;
- speech end latency;
- false positives;
- false negatives;
- minimum speech duration;
- silence thresholds;
- interaction with transcription chunking.

Avoid tuning against a single audio sample.

## Model handling

Model-related changes should account for:

- download/readiness state;
- file integrity;
- local paths;
- memory footprint;
- load/unload lifecycle;
- model compatibility;
- CPU/GPU/backend availability;
- fallback behavior.

## Cross-platform audio

Do not assume device enumeration, permissions, or audio session behavior is identical across macOS, Windows, and Linux.

Use the relevant platform specialist when native capture behavior is involved.

## Validation

Prefer:

1. small deterministic unit tests;
2. known audio fixtures;
3. affected crate tests;
4. CLI reproduction;
5. live-device testing when required.

When using fixture audio, document what behavior the fixture is intended to validate.

## Escalate

Use Performance Engineer for latency/resource optimization.

Use Rust Developer for ownership/concurrency/runtime structure.

Use Swift, Windows, or Linux specialist for native capture/session integration.
