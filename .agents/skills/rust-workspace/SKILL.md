---
name: rust-workspace
description: "Use for work on RimV Cargo workspace crates, Rust APIs, ownership, errors, features, or CLI programs. Not for changes limited to the Vite frontend."
---

# Rust Workspace Development

## Scope
`Cargo.toml`, `crates/*`, Rust applications under `apps/`, and Rust-facing native integration. Do not inspect the entire workspace before locating the requested symbol.

## Start
1. Identify the affected crate using `.aiassistance/context/PROJECT_MAP.md` and the named source file.
2. Read that crate's `Cargo.toml` for the **actual package name**, features, targets, platform dependencies, and local path dependencies.
3. Search relevant symbols and direct callers; inspect the contract before changing signatures.
4. For a workspace-boundary change, examine dependency direction and relevant feature configurations.

## Implementation rules
- Preserve existing error style and visibility conventions.
- Express invalid states with types where useful, not ad hoc flags.
- Keep blocking operations off async executor threads and audio callbacks.
- Prefer owned data only when ownership is required; document borrowed lifetimes at FFI edges.
- Guard platform-specific code with appropriate configuration and check other build targets where available.
- Use feature flags for genuine optional behavior; prevent untested accidental feature combinations.
- Treat serialized enums/structs and CLI output as external contracts.
- Avoid new dependencies unless justified by a concrete requirement.

## Validation commands
```bash
# Use the package name from the affected Cargo.toml.
cargo fmt --all -- --check
cargo check -p <package>
cargo test -p <package>
# Broader only if contracts, features, or shared types changed:
cargo test --workspace
```
`cargo clippy -p <package> --all-targets -- -D warnings` is useful where the existing workspace/toolchain supports it; do not introduce unrelated lint cleanups.

## Review of high-risk changes
- Is the error useful at the caller boundary?
- Are cancellation and shutdown paths handled?
- Are borrows/clones justified by lifetime requirements?
- Are panic conditions safe and documented?
- Are dependent crates updated if public types change?
- Have target-specific compilation assumptions been checked?

## Escalation
Use `rust-async-concurrency` for channels, tasks, locks or async shutdown; `security-privacy` for unsafe/FFI/process/path changes; `performance-profiling` for measured hot-path work; `cross-platform` for shared abstraction changes.
