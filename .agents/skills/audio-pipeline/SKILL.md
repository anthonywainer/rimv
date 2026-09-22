---
name: audio-pipeline
description: "Use for RimV microphone/audio capture, buffers, sample-rate conversion, VAD handoff and low-latency audio transport. Excludes purely visual microphone UI styling."
---

# Audio Capture and Pipeline

## Goal
Preserve sample correctness and predictable latency from device to model input across macOS, Windows and Linux.

## Trace the pipeline
```text
Device → capture API → sample format/channel mapping → buffer/queue
       → resampling/normalization → VAD/chunking → transcription input
```
Record for each edge: sample rate (Hz), channels, numeric representation, frame size, timestamp source, ownership and overflow policy.

## Procedure
1. Identify the failing or requested stage and its immediate producer/consumer; inspect only those first.
2. Verify the actual device format and the model-required format. Never assume a device provides 16 kHz mono PCM or that all OSes enumerate the same devices.
3. Make conversion policy explicit: channel downmix, resampling, clipping and silence handling.
4. Keep callback execution bounded; schedule resampling, logging, VAD and inference outside the device callback if needed.
5. Choose bounded buffering with a documented overflow response. Monitor queue depth or dropped frames if latency matters.
6. Define timestamps in consistent units; distinguish source capture time from processing completion time.
7. Handle device loss, input switch, permissions and stop/cancel cleanly.
8. Test deterministic fixture paths separately from live hardware.

## Format invariants
- Interleaved vs planar layout is explicit.
- Numeric type and normalization range are explicit (`i16`, `f32`, etc.).
- Mono conversion should be intentional, not "first channel only" without rationale.
- Chunk durations derive from sample counts and sample rate, not guessed wall time.
- Resampling preserves continuous-stream state between chunks if the algorithm requires it.

## Useful tests
- silence and near-silence;
- clipped input;
- stereo to mono;
- device format mismatch;
- final partial chunk;
- startup and immediate stop;
- device disconnect and reconnection if supported.

Use `speech-transcription` for result semantics/VAD thresholds and `performance-profiling` for measurement-based latency work. Native capture behavior belongs with the appropriate platform agent.
