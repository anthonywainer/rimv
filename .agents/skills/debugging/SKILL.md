---
name: debugging
description: "Use when a RimV bug has an uncertain cause, a failing build/test or a reproducible runtime issue. Avoid speculative multi-file edits before capturing evidence."
---

# Evidence-Driven Debugging

## Objective
Find the cause, make one focused correction and demonstrate that the originally failing behavior now works.

## Investigation
1. Capture exact symptoms: action/input, expected result, observed result, environment and exact error text.
2. Make reproduction deterministic where possible: fixture audio, test case, CLI invocation or browser interaction.
3. Locate the failing boundary using the project map, targeted symbol search and nearby tests.
4. Form a small number of falsifiable hypotheses; for each, decide what evidence would disprove it.
5. Add the smallest useful temporary instrumentation or targeted test, without logging sensitive transcript/audio by default.
6. Identify the root cause; explain why it produces the observed symptom before patching.
7. Make the minimal correction; rerun the original reproduction and relevant regression tests.
8. Remove diagnostic noise or retain only useful production observability.

## Diagnostic routing
| Symptom | Start here |
|---|---|
| Rust compile error | Exact compiler diagnostic, affected crate and feature flags |
| Audio corruption | Rate, channels, numeric format, chunk boundaries |
| Hanging shutdown | Task/channel owners, cancellation and locks |
| Missing transcript | VAD → segment → model readiness → result dispatch |
| UI stuck loading | Async response, stale state, error path and cancellation |
| Native crash | FFI ownership, ABI, callback lifetime and platform logs |
| Release-only failure | Feature/profile flags, native libraries, bundling and signing |

## Avoid
- changing five files simultaneously to test a guess;
- "fixing" crashes by suppressing errors;
- sleeping longer to hide races;
- relying on a full log dump when a specific stack trace suffices;
- claiming a fix before the failing scenario has been rerun.

## Exit
A reproducible failure now passes, an appropriate regression test exists when feasible, collateral validation passes, and any remaining environmental uncertainty is clearly stated.
