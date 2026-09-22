---
name: testing
description: "Use for Rust, TypeScript, audio-fixture, CLI, protocol and platform regression tests; test planning or fixing flaky suites. Not needed to invent tests for a purely editorial change."
---

# Risk-Based Testing for RimV

## Goal
Prove the changed behavior at the cheapest reliable layer, with realistic tests for audio, protocols and platform adapters.

## Test selection
| Change | Preferred first test |
|---|---|
| Pure algorithm/state | Unit test |
| Crate boundary | Package integration test |
| Protocol/event schema | Producer/consumer contract test |
| VAD/transcription | Pinned audio fixture and event assertions |
| Browser UI | Node test + typecheck; browser check when necessary |
| Native permissions/audio | Platform integration/manual device check |
| CLI | Execute command against controlled input |
| Release packaging | Package smoke test on target OS |

## Procedure
1. Capture user-observable acceptance criteria and important failure cases.
2. Reproduce the reported bug with a failing test where practical.
3. Keep fixtures deterministic and local by default; do not require network or a real mic for ordinary CI.
4. Isolate platform-dependent tests and label environment requirements.
5. Test success, malformed input, empty input, cancellation and cleanup where relevant.
6. Prefer assertions on behavior/events rather than implementation internals.
7. Run the narrowest failing test first, then affected package/app, then larger integration scope only if needed.

## Concrete commands
```bash
cargo test -p <actual-package-name>  # read Cargo.toml first
cargo test --workspace               # when cross-crate impact warrants it
cd apps/web && npm run test && npm run typecheck
```
The current web `lint` script is typecheck, not an ESLint scan.

## Audio test caution
Exact transcript words may vary by model/version/backend. Where relevant, lock the model and decoding settings or assert stable properties: no output for silence, nonempty final transcript for controlled speech, monotonic timestamps, bounded events, no duplicate finalization.

## Flaky tests
Find reliance on sleeps, time, unordered concurrency, global state, hardware, live networks and hidden files. Fix the nondeterministic condition instead of blindly increasing timeouts.

## Exit
The regression is covered when practical, relevant checks pass, and unrun device/platform tests are identified without claiming verification.
