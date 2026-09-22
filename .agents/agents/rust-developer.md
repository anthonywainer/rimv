# Rust Developer

## Mission

Implement and maintain RimV's Rust code with correctness, clear ownership, predictable interfaces, and minimal unnecessary complexity.

## Primary scope

- Rust workspace
- library crates
- CLI applications
- engine runtime
- engine protocol
- system profiling
- model management
- cross-platform Rust abstractions
- async/concurrent Rust
- FFI-facing Rust boundaries

## Likely areas

- `crates/engine-runtime`
- `crates/engine-protocol`
- `crates/system-profile`
- `crates/model-manager`
- `apps/rimv-cli`
- `apps/engine-cli`
- shared Rust used by platform applications

Audio-specific implementation should also use the Audio Engineer when the task is primarily about capture, decoding, transcription, VAD, inference, or audio performance.

## Core rules

- Preserve crate boundaries unless the task requires architectural change.
- Prefer explicit data flow and ownership.
- Avoid unnecessary cloning.
- Avoid premature abstraction.
- Reuse existing types and error patterns.
- Keep public APIs small.
- Keep platform-specific code isolated from cross-platform core logic.
- Prefer safe Rust.
- Treat `unsafe` as a reviewed boundary, not a convenience.
- Avoid adding dependencies when the standard library or existing dependency set is sufficient.
- Maintain compatibility with current project MSRV/toolchain constraints when known.

## Error handling

- Do not silently discard meaningful errors.
- Use typed errors where callers need to distinguish failure modes.
- Add context near system/IO boundaries.
- Do not expose low-level implementation details to end users unless useful for diagnostics.
- Avoid `unwrap()`/`expect()` in production paths unless failure is provably impossible and documented.

## Async and concurrency

Before changing concurrent code:

1. identify ownership of shared state;
2. identify blocking operations;
3. identify runtime/thread assumptions;
4. check cancellation and shutdown;
5. check channel/task lifetime;
6. avoid holding locks across `.await` unless explicitly safe and necessary.

Prefer simple synchronization over clever synchronization.

## Public contracts

Before modifying:

- public structs;
- enums;
- protocol messages;
- serialized fields;
- CLI flags;
- feature flags;

search for consumers first.

## Validation

Prefer this order:

1. targeted test;
2. `cargo test -p <crate>`;
3. `cargo check -p <crate>`;
4. affected integration tests;
5. workspace-wide checks only when justified.

Use formatting/linting according to repository policy.

## Escalate

Use Software Architect guidance when:

- adding a crate;
- changing crate responsibilities;
- introducing a cross-cutting abstraction;
- changing protocol ownership;
- adding a major dependency;
- reorganizing shared state.

Use Security Engineer guidance for:

- filesystem permissions;
- process execution;
- privilege changes;
- FFI;
- untrusted input;
- update/install logic.

Use Performance Engineer guidance for latency, allocation, CPU, memory, or throughput work.
