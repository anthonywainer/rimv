---
name: speech-transcription
description: "Use for RimV speech segmentation, VAD decisions, partial/final transcripts, Whisper integration, timestamps and transcription correctness. Do not trigger for frontend transcript styling alone."
---

# Speech Transcription and VAD

## Goal
Transform audio into trustworthy, ordered transcript events while managing model readiness, latency and interrupted sessions.

## Event contract first
Specify whether an event is partial, revised, final or failed. Define session/segment identity, timestamps, language, cancellation behavior and whether a final result may supersede partial text.

## Implementation sequence
1. Reproduce using one known recording in `resources/audio/` or another approved fixture.
2. Verify input format, segmentation, VAD and inference configuration before adjusting text post-processing.
3. Confirm VAD start/end thresholds, hangover/silence duration and minimum speech duration match the intended interaction.
4. Define overlap policy for chunk boundaries. Prevent duplicate text when reusing context or overlapping windows.
5. Preserve timestamp units and monotonic ordering. A transcript should not appear earlier than its captured audio without an explicit correction event.
6. Distinguish model-not-ready, no-speech, capture-failed, decoder-failed, cancelled and timeout states.
7. Test finalization when stopping mid-speech and when the last buffer is shorter than a normal chunk.

## Model/decoder checks
- model type and language capability;
- expected sample rate and audio tensor shape;
- device/backend availability and fallback;
- streaming vs batch API semantics;
- loading and warm-up state;
- deterministic vs nondeterministic output tolerances.

## Testing guidance
Use fixture-based assertions that survive small model/runtime variation: event counts, state ordering, time ranges, nonempty text for stable speech, and no transcript for reliable silence. Avoid exact word-for-word expectations unless model, runtime, decoding config and fixture are pinned.

## Privacy and UX
No-speech must not be reported as a crash; microphone permission failure must not resemble a model failure. Clearly distinguish live/incomplete words from finalized content in UI. Do not log raw private speech unless explicitly required and protected.

## Done when
Known fixtures cover the change, final/partial semantics are intact, stop/cancel releases resources, and no silent loss or duplicate segment is introduced.
