# RimV Native Windows v0.1 — Phase 3 Quality Report

Date: 2026-09-27

## Outcome

Phase 3 reduces production/test coupling and splits Windows application policy from WinUI and raw FFI bindings. It adds Rust and .NET test suites around the real shared-core boundaries. The portable .NET application library and the thin C# FFI adapter compile on macOS. Full Phase 3 acceptance is **not yet verified**: this machine cannot restore the .NET test packages, build or launch WinUI 3, run Windows UI automation, or run the Rust FFI integration tests because upstream Apple Swift build scripts fail in the restricted environment. No Windows runtime, audio, accessibility, scaling, or native window lifecycle tests are claimed as passed.

## Audit findings and priority

| Priority | Finding | Change |
| --- | --- | --- |
| P1 | `MockAsrBackend` was compiled as a normal public type in the production speech crate although it served only tests. | Moved it into `#[cfg(test)]` support; production builds no longer compile that mock. |
| P1 | Windows `AppCoordinator` was a 449-line WinUI service combining shared-core calls, user preference IO, event polling, dispatcher access and theme handling. | Moved core coordination into a platform-neutral .NET application library with injected core factory, preferences store and UI dispatcher. Moved theme handling into the WinUI layer. Commands are serialized and shutdown joins the event-poll task. |
| P1 | The C# raw P/Invoke declarations, UTF-8 allocations, JSON envelope decoding and shared DTOs were combined in `CoreClient.cs`. | DTOs/contracts now live in the application library; raw imports are isolated in `CoreNativeMethods`; `CoreClient` owns only the managed FFI adapter responsibilities. |
| P1 | Neither C# app policy nor native ABI had a test project. | Added standalone .NET unit and native ABI integration test projects. |
| P2 | Rust FFI, model catalog and persisted recording/export tests were inline in production source. | Moved cross-module coverage into the respective crate `tests/` directories. Small privacy-dependent unit tests remain adjacent under test-only modules. |
| P2 | Windows coordinator tried to infer model selection from runtime state using model-path logic also owned by Rust. | Removed that inference. Windows uses saved selection state and the shared core remains authoritative for model selection and validation. |
| P2 | No deterministic prerecorded ASR fixture test existed. | Added a mono 16 kHz fixture and an ignored integration test against actual Parakeet and Silero assets. It is not a mock test and requires model assets to run. |
| P2 | Windows behaviors and UI could not be validated on the prior development path. | Added portable policy/coordinator tests and documented Windows execution requirements. Actual Windows validation remains outstanding. |

There was no code evidence of duplicated ASR, model loading, session persistence, metadata or export implementation in the native Windows UI. Those remain in the Rust core. WinUI contains presentation and Windows integration, with a narrow managed interface to the shared Rust C ABI. The current phase has not altered macOS UI or transcription behavior, nor removed the separate HTML Windows app.

## Resulting production boundaries

```text
WinUI 3 presentation and native Windows services
    └── RimV.Windows.Application (portable .NET coordination, DTOs, policy)
          └── ISharedCoreClient
                └── CoreClient (managed UTF-8/JSON ownership adapter)
                      └── CoreNativeMethods (C ABI imports)
                            └── rimv_core_ffi → shared Rust engine/core
```

The .NET application layer has no WinUI or Windows App SDK package dependency. `ISharedCoreClientFactory`, `ISharedCoreClient`, `IUserPreferencesStore` and `IUiDispatcher` are injected at the composition root. This provides test seams at real boundaries without creating an IoC framework. The WinUI project still owns windows, theme integration and platform-specific dispatch. The `CoreClient` remains responsible for Rust handle/string lifetimes and serialized request/response conversion.

Test-only mocks live in test projects or `#[cfg(test)]` Rust support. The new real-audio test is opt-in and does not ship in the application. .NET test dependencies are scoped to test csprojs. The WAV fixture is ignored by the repository's broad `*.wav` rule and must be explicitly included when staging source; build outputs are ignored with `bin/` and `obj/` rules.

## Automated tests added

| Suite | Actual coverage |
| --- | --- |
| `crates/model-manager/tests/catalog.rs` | Catalog lookup, language/backend support and selection validation. |
| `crates/engine-runtime/tests/recordings.rs` | Persisted recording details, rename/delete, TXT and JSON export, invalid identifiers/path traversal. |
| `crates/rimv-ffi/tests/ffi_contract.rs` | Actual exported ABI lifecycle, API version, JSON request/error envelopes, event poll, capability state, invalid config/export and null inputs. |
| `crates/speech-transcription/src/tests.rs` | Existing private unit behavior remains test-only in the owning module. |
| `crates/speech-transcription/tests/asr_audio_fixture.rs` | Actual deterministic audio through Parakeet + Silero; ignored unless the model/VAD asset environment variables are set. |
| `apps/windows-native.tests/` | Portable source-selection policy, start/stop commands, saved selection compatibility, event delivery and shutdown, preferences, install/delete guards. |
| `apps/windows-native.integration-tests/` | Actual Windows DLL API lifecycle when `RIMV_CORE_FFI_DLL` points to a built `rimv_core_ffi.dll`. |

No numeric code coverage is reported. `cargo llvm-cov` was not available, and Coverlet could not execute because test package restore was blocked. The matrix above describes test intent and source coverage, not pass status.

## Validation performed

Passed:

- `cargo fmt --all -- --check` using the configured Rust toolchain.
- `cargo test --offline -p model-manager`: four catalog integration tests passed.
- `cargo test --locked --offline -p speech-transcription --no-default-features`: 22 unit tests passed.
- `cargo clippy --locked --offline -p speech-transcription --no-default-features --all-targets -- -D warnings` passed.
- `cargo clippy --locked --offline -p model-manager --all-targets -- -D warnings` passed.
- `dotnet build apps/windows-native-application/RimV.Windows.Application.csproj --ignore-failed-sources -p:NuGetAudit=false`: passed with zero warnings/errors.
- Isolated compilation of `CoreClient`, `CoreNativeMethods` and `CoreClientFactory` against the application library: passed with zero warnings/errors.
- `git diff --check` passed at the time of review.

Attempted but blocked:

- `cargo test --locked --offline -p rimv-ffi` and `cargo test --locked --offline -p engine-runtime --test recordings`: native Apple dependency build scripts fail before crate tests. `screencapturekit`'s bundled SwiftLint rejects warnings in its own Swift sources; the Apple Swift bridge also cannot write the clang module cache under this sandbox. These failures are outside changed project code. Consequently the newly added FFI and recording integration tests have not executed here.
- `dotnet test apps/windows-native.tests/RimV.Windows.UnitTests.csproj`: NuGet restore cannot reach `api.nuget.org`, and the packages (`xunit`, `Microsoft.NET.Test.Sdk`, `coverlet.collector`) are not present in the local cache.
- Native Windows integration test project: not executed; it requires Windows and the compiled DLL.
- WinUI application build and launch: not executed; Windows App SDK/WinUI runtime is unavailable on this macOS host, and NuGet access is blocked.
- Audio capture, tray placement, native window reopen/close/shutdown, duplicate event observation, keyboard/screen-reader behavior, light/dark appearance and display scaling: require a Windows device or runner and remain pending.
- ASR audio fixture: not executed; Parakeet and Silero model assets are not installed in this environment.
- Full workspace/macOS regression suite: not demonstrated in this phase because the same Apple dependency build failures stop compilation before tests. No macOS application source was changed.

## UI quality review

The existing WinUI screens were reviewed against `SPEC-01`, `SPEC-02`, `SPEC-04`, the RimV design system, Windows platform guidance and usability heuristics. This phase makes the theme manager a UI-layer service, routes model-state actions through shared policy, and keeps presentation windows bound to coordinator state/events. The application coordinator no longer reaches directly for global WinUI dispatch or theme APIs.

This was a source-level quality review, not an interactive usability or accessibility validation. No claims are made about Narrator, keyboard-only flows, high-DPI rendering or light/dark appearance on Windows. Those must be validated on the actual native shell before Phase 3 UI sign-off.

## Skills and guidance applied

Project skill files read and applied:

- `.agents/skills/clean-code/SKILL.md`
- `.agents/skills/solid-design/SKILL.md`
- `.agents/skills/rust-workspace/SKILL.md`
- `.agents/skills/windows-native/SKILL.md`
- `.agents/skills/windows-ui/SKILL.md`
- `.agents/skills/testing/SKILL.md`
- `.agents/skills/ui-design/SKILL.md`
- `.agents/skills/usability-heuristics/SKILL.md`
- `.agents/skills/accessibility/SKILL.md`
- `.agents/skills/api-contracts/SKILL.md`
- `.agents/skills/rust-async-concurrency/SKILL.md`
- `.agents/skills/security-privacy/SKILL.md`

Specialist guidance read: `.agents/agents/software-architect.md`, `.agents/agents/windows-developer.md`, `.agents/agents/rust-developer.md`, `.agents/agents/test-engineer.md` and `.agents/agents/ui-ux-designer.md`. Design references read: `.agents/design/RIMV_DESIGN_SYSTEM.md`, `.agents/design/platforms/windows/WINUI_IMPLEMENTATION_GUIDE.md`, `.agents/design/platforms/windows/WINDOWS_UI_REVIEW_CHECKLIST.md`, `.agents/design/platforms/windows/WINDOWS_PLATFORM_GUIDELINES.md` and the linked usability heuristic references.

The project does not contain a standalone TDD skill or a standalone C# skill. Test-first practices were applied using the testing skill; C# and WinUI practices followed the Windows UI/native guidance and existing project conventions. No nonexistent skills are represented as applied.

## Remaining work / sign-off gates

1. Run .NET restore, unit tests and Coverlet on a network-enabled Windows CI runner.
2. Run the actual Windows DLL integration test after building the Rust FFI for Windows.
3. Build and launch the WinUI app on Windows; validate tray, independent windows, app shutdown and Rust event handling.
4. Complete keyboard, Narrator, contrast, theme and high-DPI checks on Windows.
5. Run the deterministic ASR fixture test where the pinned model/VAD assets are available.
6. Re-run macOS regression tests in an environment where Apple Swift dependency build scripts work.

No installer work, release, or version tag is included in this phase.
