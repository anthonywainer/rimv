---
name: solid-design
description: "Use when designing or refactoring interfaces and module responsibilities across Rust, Swift or TypeScript. Do not apply SOLID mechanically to small functions or data types."
---

# SOLID and Pragmatic Modularity

## Goal
Use SOLID as a set of diagnostic questions, not a checklist that demands an interface/trait for every function. The smallest maintainable solution wins.

## Five principles in RimV

### Single Responsibility
Ask what **kind of change** should cause a module to change. Separate audio capture, transcription policy, model lifecycle, protocol serialization and UI state when they have independent reasons to change. Do not split one coherent unit just because it has many lines.

### Open/Closed
Prefer adding a new platform adapter or model backend through an existing stable extension point. Do not introduce plugin machinery until two or more implementations demonstrate a real variation point.

### Liskov Substitution
For any Rust trait implementation or Swift protocol conformance, document preconditions, failures, cancellation, and ownership. An implementation must not unexpectedly narrow the promised behavior; a mock should honor the same contract.

### Interface Segregation
Prefer narrow traits/protocols representing real consumers, such as read-only model metadata versus model installation. Do not create one giant engine service interface for the CLI, web UI, and native UI.

### Dependency Inversion
High-level transcription orchestration should depend on a useful abstraction when the platform or inference backend genuinely varies. Keep `crates/engine-protocol` independent from frontend/native UI implementation details. Use concrete types by default within stable local modules.

## Change procedure
1. Identify the consumer and the specific change pressure.
2. Draw the dependency direction from core → contract → adapter; inspect current imports/dependencies.
3. Propose the smallest meaningful boundary (sometimes a normal function suffices).
4. Specify failure behavior and data ownership at the boundary.
5. Implement one vertical use case; avoid creating unused extension points.
6. Test the implementation through observable behavior and one other implementation only if it exists.

## Cross-platform example
A microphone adapter may differ by platform, but the shared audio pipeline should consume a stable chunk representation with explicit sample rate and timestamp semantics. Put platform permission prompting in the UI/native adapter, not the shared DSP core.

## Avoid
- a trait per struct, interface per file, generic layers with one implementation;
- inheritance-like indirection when a Rust enum is clearer;
- dependency injection frameworks without demonstrated need;
- "future-proofing" a hypothetical platform feature while the present contract is unclear.

## Acceptance
The dependency direction is legible, the new abstraction has real clients, the public contract is documented, and no unrelated layers were introduced.
