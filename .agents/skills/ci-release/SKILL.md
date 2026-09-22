---
name: ci-release
description: "Use for RimV GitHub Actions, Makefile, Cargo/Vite CI, cross-platform build matrices, native dependencies, signing and distribution scripts. Not for ordinary feature work."
---

# CI and Release Engineering

## Scope
`.github/workflows/`, `Makefile`, `.cargo/config.toml`, packaging scripts and application build pipelines. RimV includes existing macOS staging/signing/notarization scripts but must not be treated as macOS-only.

## Change procedure
1. Identify which event triggers the pipeline (push, PR, tag, manual) and which platform/architecture it targets.
2. Confirm actual build inputs: Rust toolchain/features, Node version, model/native runtime dependencies and cache keys.
3. Keep checks layered: format/typecheck/unit test before expensive packaging where possible.
4. Define artifact identity (version, OS, architecture, build profile, dependency set) explicitly.
5. Never log signing tokens, API keys, private certificates or notarization secrets.
6. Keep signing/notarization separate from untrusted PR code execution.
7. Verify partial failure behavior: cleanup, artifact retention, idempotent rerun and safe rollback when applicable.
8. Validate generated artifacts on each actual supported platform when possible; report unavailable targets.

## RimV-specific concerns
- Cargo workspace may have native ONNX/sherpa dependencies; verify library inclusion and architecture.
- Vite's build includes TypeScript checking; a passing web build does not validate native engine behavior.
- Model assets may be large; explicitly decide whether they are embedded, downloaded or staged.
- Windows DLL loading, macOS dylib/rpath and Linux shared-object dependencies require platform-specific packaging checks.
- Installer/update behavior must preserve user data and not falsely report success after a partial download.

## Release checklist
- [ ] Version agrees across relevant manifests/artifact names.
- [ ] Intended platform/architecture build is reproducible enough to rerun.
- [ ] Tests relevant to the release path pass.
- [ ] Native runtime dependencies are bundled or documented.
- [ ] Secrets stay in protected CI variables and are not echoed.
- [ ] Signed/unsigned artifact status is accurate.
- [ ] Install, first run, microphone permission, model setup and uninstall/smoke test have been exercised where practical.

## Exit
Changed workflow is syntactically valid, platform-specific effects are understood, and actual CI/packaging checks run are reported without overstating validation.
