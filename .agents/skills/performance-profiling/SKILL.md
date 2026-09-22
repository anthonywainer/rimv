---
name: performance-profiling
description: "Use for measured RimV latency, memory, CPU, startup, UI responsiveness, model loading and audio throughput bottlenecks. Do not optimize merely because code looks slow."
---

# Measurement-Based Performance

## Goal
Improve a named metric under a reproducible workload while preserving correctness and acceptable resource use.

## Establish a baseline
1. Define the user-visible symptom and metric: capture-to-text latency, dropped frames, inference time, startup, peak RSS, CPU, UI frame responsiveness or binary size.
2. Specify the workload: audio format, sample length, model/version, backend, hardware, app build profile and concurrent load.
3. Collect repeated measurements. Distinguish cold start, warm steady state and tail latency if meaningful.
4. Instrument one stage at a time: capture, queue, resampling, VAD, inference, event transport, UI rendering.
5. Confirm the bottleneck with profiling, not intuition. On macOS, Instruments may help; use platform-appropriate profilers elsewhere.

## Optimization decision order
- Eliminate accidental repeated work, blocking calls and unnecessary copies first.
- Consider buffer reuse and backpressure where measurement shows pressure.
- Tune batch/chunk size against both throughput **and** end-to-end latency.
- Evaluate model variants/backends against accuracy and memory requirements.
- Use `unsafe`, lock-free structures or SIMD only if benefits are measured and invariants are testable.

## Benchmark discipline
Keep warm-up, input, thread settings, temperature/power mode and external load as comparable as practical. Report units and sample size; a single best run is not a reliable baseline.

## Audio-specific metrics
- callback budget and overruns;
- input queue depth and drop count;
- VAD start/end delay;
- first partial and final transcript latency;
- real-time factor;
- model cold-load and peak memory.

## UI-specific metrics
- time to first useful interface;
- input-to-visible-feedback latency;
- large transcript rendering;
- avoid long synchronous work in the main/browser thread.

## Exit report
Record before/after measurement, workload, platform, tradeoffs and relevant correctness-test results. If no reliable baseline is available, treat the optimization as a hypothesis, not a proven improvement.
