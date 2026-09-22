---
name: api-contracts
description: "Use for changes to engine-protocol, IPC messages, JSON/HTTP interfaces, CLI flags, native bridge contracts or frontend/backend integration across RimV components."
---

# Contracts and Integration Boundaries

## Goal
Evolve shared interfaces deliberately so producers and consumers agree on format, lifecycle and errors.

## Inventory the contract
Determine whether it is:
- Rust public API across workspace crates;
- engine IPC messages;
- JSON/HTTP payloads;
- CLI command arguments/output;
- Swift/Windows/Linux FFI boundary;
- UI event stream (partial/final/live status).

## Procedure
1. Locate the authoritative definition (often `crates/engine-protocol`, but verify source).
2. Search all producers and consumers before editing. Include `apps/web` and native apps when relevant.
3. Write down message/schema, allowed states, null/optional semantics, errors, version expectations and ordering requirements.
4. Classify change as additive/compatible, behavior-changing or breaking. Do not assume an added enum variant is harmless to exhaustive clients.
5. Update both ends or define a transition/migration behavior. Preserve unknown-field compatibility only if implementation genuinely supports it.
6. Add tests for serialization, round trips, malformed inputs, unknown versions/states where appropriate, and end-to-end event ordering when needed.
7. Update only the directly affected contract documentation and examples.

## Audio event example
Partial and final transcript events should have explicit session/segment identity, clear revision/finalization semantics, and consistent timestamp units. UI consumers must not append all partials as permanent transcript text.

## Native bridge example
Explicitly define buffer ownership, lifetime, C ABI representation, thread assumptions, error translation and cancellation. Rust safety guarantees do not extend through unmanaged foreign pointers.

## Compatibility checklist
- Default or missing fields are well-defined.
- Old stored messages/clients do not silently change meaning.
- Errors are actionable without leaking private data.
- Producers and consumers agree about cancellation, disconnect and retries.
- Feature flags and target-specific implementations remain coherent.

## Exit
All affected contract consumers are updated or a compatibility path is documented, relevant tests pass, and any intentionally breaking change is called out.
