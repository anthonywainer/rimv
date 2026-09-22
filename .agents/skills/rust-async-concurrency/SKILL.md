---
name: rust-async-concurrency
description: "Use for Rust async tasks, threads, mutexes, channels, audio callback handoff, cancellation and shutdown problems. Do not trigger for ordinary synchronous code."
---

# Rust Async and Concurrency

## Goal
Correct task lifetime, predictable cancellation, bounded buffering and no unintended blocking in RimV's engine and audio paths.

## Pre-change model
Identify the following *before* editing concurrent code:

| Concern | Questions |
|---|---|
| Producers/consumers | Who owns each sender and receiver? One or many? |
| Execution | OS thread, runtime task, or realtime audio callback? |
| Buffering | Bounded/unbounded? What happens on overflow? |
| Ownership | Which task owns shutdown and joins child tasks? |
| Cancellation | How do we interrupt pending IO/inference? |
| Locking | Who shares which lock, and in what order? |
| Ordering | Must chunk/transcript events be ordered? |

## Safe sequence
1. Reproduce or specify the state transition (start → active → stop/cancel → shutdown).
2. Find channel/task creation and all exit paths; search sender clones and drop behavior.
3. Prefer bounded queues at real-time ingress; document backpressure/drop semantics.
4. Make cancellation observable and idempotent; distinguish graceful completion from interruption.
5. Ensure tasks are awaited, joined or intentionally detached with a documented lifetime.
6. Keep blocking operations away from async executor threads; move inference/decoding to appropriate workers.
7. Add tests for the failure mode, especially shutdown when idle, full queue, dropped consumer or error during startup.

## Audio callback invariant
Keep callbacks bounded. Avoid disk/network IO, model inference, blocking mutex waits, large allocations and log flooding inside callbacks. Pass data to a worker through an explicitly bounded path.

## Common failure modes
- lock held across `.await`;
- unbounded queue that grows during slow inference;
- channel never closes because a sender clone survives;
- child task outlives engine state;
- cancellation leaves device/session busy;
- converting thread-safety problems into unexplained `unsafe` blocks;
- synchronous callback waiting for async work.

## Tests
Use deterministic synchronization instead of sleeping when practical. Cover normal stop, stop during startup, double stop, worker failure, dropped receiver and queue saturation. Use platform integration tests for actual device shutdown.

## Exit check
No orphan tasks or leaked handles, no known deadlock or unbounded queue, state transitions are explicit, and the relevant concurrency tests pass.
