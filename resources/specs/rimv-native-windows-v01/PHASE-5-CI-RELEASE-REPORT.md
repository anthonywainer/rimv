# RimV Native Windows v0.1 — Phase 5 CI and release report

Date: 2026-09-27

## Baseline captured before workflow changes

The source baseline is commit `c38cffa`. Historical Actions runs and step timings were inaccessible: the repository is private from the available web tool, and `gh auth status` reported no GitHub login. Duration and cache-hit comparisons below must be filled from actual Actions runs; no timing improvement is claimed from YAML inspection.

| Workflow | Before Phase 5 | Cost or release concern |
| --- | --- | --- |
| `ci.yml` | Push/PR change detection, Linux workspace check/clippy/test, web build, push-only macOS workspace check/build, Windows workspace check/build plus Tauri CLI install, runtime staging and HTML installer build | Windows build repeated Rust work and compiled legacy Tauri even for the native release; shared Rust changes did not consistently trigger macOS and native Windows checks. |
| `cross-platform.yml` | Manual Linux/macOS/Windows workspace format/check/clippy/test | Useful broad coverage, but does not build the WinUI app or installer. |
| `release.yml` | Tag/manual Linux workspace quality, macOS and Windows CLI archives, separate Tauri installer, checksum job, GitHub Release | Publishes the HTML/Tauri Windows installer and Windows CLI archives; never validates the native installer from a fresh runner. |
| `windows-native-package.yml` | Manual/PR native Rust and .NET tests, full native installer build, artifact upload, separate Windows install smoke test | Expensive packaging on every matching PR; no Rust compiler-output or NuGet cache; Phase 4 package builder deliberately starts from a clean Cargo target to avoid stale DLLs. |

The native WinUI host calls `rimv_core_ffi.dll`; shared ASR, model, session and export logic remains in `engine-runtime`, `model-manager` and related Rust crates. C# test projects and the PowerShell package harness are outside the production WinUI project. A deterministic prerecorded ASR fixture exists, but its real-model test is ignored unless Parakeet and Silero assets are supplied; no CI job can claim it passed without those assets.

## Optimized workflow design

- `ci.yml` keeps Linux workspace tests, removes the automatic Windows Tauri CLI install/HTML installer build, and runs focused macOS shared-core tests and `make menu-test` for both PRs and pushes that change shared Rust or macOS code. Windows-only packaging changes skip the macOS rebuild. Its push runs are never canceled; superseded PR runs are canceled.
- `ci.yml` calls `windows-native-package.yml` as a reusable workflow for native Windows changes. This gives each push one top-level CI run. The called workflow runs format/lint/shared-core and C# unit/native frontend compilation on PRs. On main pushes or manual CI dispatch it builds the native WinUI/NSIS installer, tests the real Windows FFI, scans the production payload, uploads the exact artifact in the CI run, and installs that artifact on a separate Windows runner. It has bounded job and installer timeouts. No job invokes Tauri.
- Windows Rust builds use pinned Rust 1.98.1, Cargo registry caching and `sccache` for compiler results. The package builder still removes its Cargo output tree before every build so old profile DLLs cannot enter the installer; compiler outputs may be reused through sccache. The workflow reports cache stats after execution.
- The WinUI project and both C# test projects now have generated `packages.lock.json` files. NuGet caching keys off those lockfiles; all restore operations use locked mode. The package builder restores the exact `win-x64` self-contained configuration once, then publishes with `--no-restore`.
- `cross-platform.yml` remains a manual broad Linux/macOS/Windows workspace suite, with Rust pinned to 1.98.1. Its legacy workspace coverage remains available without entering the native release path.
- `release.yml` uses a successful `ci.yml` push or manual run **for the exact tag commit** as its quality gate. Version-manifest changes and manual CI runs schedule the native Windows jobs, so CI success includes the fresh-runner installer lifecycle. Release downloads that run's validated installer and verifies its manifest and SHA-256. It retains the existing macOS arm64 Capture/Transcribe/Server archives, omits Windows CLI archives and all Tauri artifacts, and publishes only the native NSIS installer plus a checksum as Windows assets. It does not recompile Windows code.
- A tag publication requires a committed manual Windows acceptance report with explicit PASS results for installation, tray, accessibility/scaling, Microphone, System, Both, recordings/exports and graceful shutdown. The workflow refuses existing historical releases.

## Before/after performance measurements

| Measurement | Previous pipeline | Optimized pipeline |
| --- | --- | --- |
| Full workflow wall time | Unavailable: private Actions run history is inaccessible without GitHub authentication | Pending first successful remote run |
| Windows Rust compilation duration | Unavailable | Pending GitHub step timings |
| Rust compiler cache effectiveness | No native Windows compiler-output cache; previous `rust-cache` configuration disabled target caching | `sccache` hit/miss totals pending first and repeated runs |
| NuGet cache effectiveness | No native .NET cache or committed lockfiles | Lockfile-keyed cache hit/miss pending GitHub run |
| Release Tauri CLI install + HTML installer build | Two explicit Windows workflow paths (`ci.yml` and `release.yml`) | Zero native/release paths |
| Release Windows CLI archive job | One Windows matrix entry | Removed from new native release |
| Release quality/re-download stages | Linux quality job plus checksum upload and release re-download | Prior successful CI required; checksum and publish happen in one assemble job |

When a GitHub run becomes accessible, record its URL, commit SHA, runner image, each step duration, total wall time, `sccache` stats, NuGet hit/miss, tests passed/failed, and artifact SHA-256. Compare observed values with a prior run if that history becomes accessible. No speedup is claimed from the workflow definition alone.

## Validation and release gates

Local validation is recorded below. The native Windows prerelease must contain only the validated native installer and its checksum as Windows assets. Interactive tray, GUI usability, graceful user-driven shutdown and real microphone/system-audio capture require separate Windows desktop/hardware evidence. The existing `v0.1.0-beta` and `v0.1.0-beta.1` tags and releases remain unchanged.

Passed on the macOS development host:

- `actionlint` 1.7.12: all four workflow files parsed and passed workflow/expression checks. The downloaded binary matched the upstream SHA-256 shown on its release page.
- PowerShell parser: native package builder and installer test harness passed. NuGet locked restore succeeded for the WinUI `win-x64` self-contained configuration and both C# test projects.
- `cargo fmt --all -- --check` and focused `cargo clippy ... -D warnings` passed.
- Rust `app-paths`, `model-manager` and `speech-transcription` tests: 27 passed. The real prerecorded Parakeet/Silero ASR fixture passed with the installed local models (1 integration test).
- Native macOS `make menu-test`: built the existing menu app and reported `Native menu self-test: PASS` when run with macOS AppKit access outside the sandbox.
- C# application unit tests: 24 passed. The C# FFI integration test passed against a temporary copy of the real macOS Rust library and adjacent ASR dylibs (1 test); the temporary copy received a test-only loader rpath.
- `git diff --check` passed before staging.

Unavailable or pending:

- WinUI compilation on this Mac stops when the Windows App SDK invokes `XamlCompiler.exe`, a Windows binary. No Windows executable or NSIS installer was produced locally.
- No Windows x64 Rust/.NET build, packaged DLL scan, separate-runner installation, GUI tray interaction, graceful user Quit or physical Windows audio test has run for this commit yet.
- The real ASR fixture is not run in GitHub CI because the 640 MB local Parakeet model and Silero model are not CI inputs. The manual physical-audio release gate remains mandatory; the local fixture result is not represented as a Windows CI result.

GitHub CLI/API authentication is unavailable in this session (`gh auth status` failed), and web access to the private repository's Actions history returned 404. Git SSH access is available for a push, but it does not expose workflow logs. A workflow URL, Windows installer artifact, new prerelease tag and download URL must not be reported until GitHub confirms them. The worktree's unrelated, pre-existing deletion of `resources/specs/RimV_Windows_v0.1_Roadmap.md` is excluded from the Phase 5 commit.

The next unused local prerelease tag is `v0.1.0-beta.2`. It remains a proposal until the Cargo and WinUI versions are updated together, a manual acceptance report is committed, and successful CI/native Windows runs are visible for that exact version commit. No tag or release is created in this phase without those gates.

## Skills applied

- `.agents/skills/ci-release/SKILL.md`
- `.agents/skills/clean-code/SKILL.md`
- `.agents/skills/rust-workspace/SKILL.md`
- `.agents/skills/windows-native/SKILL.md`
- `.agents/skills/testing/SKILL.md`
- `.agents/skills/security-privacy/SKILL.md`
- `.agents/skills/cross-platform/SKILL.md`

No dedicated C#/.NET, GitHub Actions, Windows installer or standalone end-to-end testing skill exists in `.agents/skills/`; the listed project skills cover those procedures. `.agents/agents/devops-engineer.md` was used as specialist guidance, without spawning another agent.
