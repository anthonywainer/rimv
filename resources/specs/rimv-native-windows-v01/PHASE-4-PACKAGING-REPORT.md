# RimV Native Windows v0.1 — Phase 4 Packaging Report

Date: 2026-09-27

## Status

The reproducible Windows x64 packaging path, NSIS installer source, a separate package/install validation harness, CI workflow and install/upgrade guidance are implemented. **The installer itself has not been generated or tested on Windows from this macOS development host. Phase 4 is not signed off.** The new workflow builds and uploads a short-lived workflow artifact for test only; it creates no GitHub Release or tag.

## Packaging audit

| Area | Repository evidence |
| --- | --- |
| Native frontend | `apps/windows-native/RimV.Windows.csproj`: WinUI 3, `net10.0-windows10.0.26100.0`, `WinExe`, minimum Windows 10 build 19041, `WindowsPackageType=None`, currently framework-dependent Windows App SDK, x64 and ARM64 project platforms. The v0.1 installer intentionally supports x64 only. |
| Native Rust boundary | `crates/rimv-ffi` produces `rimv_core_ffi.dll` as a C ABI `cdylib`; app checks API version 1. `engine-runtime` enables shared sherpa-onnx/Parakeet/VAD on Windows. Whisper is excluded on Windows by the current target dependency configuration. |
| Native runtime dependencies | `sherpa-onnx-sys` copies Windows shared runtime DLLs into Cargo profile output; the package builder stages all x64 Cargo DLLs, self-contained Windows App SDK/.NET publish output, and x64 app-local VC++ redistributable DLLs. The package test checks PE machine type and `dumpbin /DEPENDENTS` references. The exact Windows output inventory has not yet been observed on a Windows runner. |
| App resources | `apps/windows/icons/icon.ico` is the existing RimV icon used by the WinUI project and the new installer. `TrayIcon` loads `Assets/rimv.ico` beside the installed application. The MIT `LICENSE` is included in the installed payload. |
| Writable state | `App.xaml.cs` uses `%LOCALAPPDATA%\RimV`; recordings, models and preferences remain outside the install folder. This lets normal per-user uninstall leave recordings and model downloads in place. |
| Existing Windows installer | `apps/windows/tauri.conf.json` configures the separate HTML/Tauri app for NSIS, current-user installation, WebView bootstrapper and legacy runtime resources. `.github/workflows/release.yml` packages that app. `scripts/windows/prepare-windows-runtime.ps1` stages legacy runtime DLLs into `apps/windows/runtime`. These paths are preserved and untouched by the new native package builder. |
| Earlier architecture plan | The Phase 0 roadmap/audit proposed Tauri while the actual later Phase 2 implementation created a distinct WinUI 3 unpackaged app. The installer decision here follows the current app, not the earlier proposal. The roadmap file is already deleted in the worktree; it was inspected from `HEAD` and its deletion is not part of this phase. |

## Installer decision

**Format: NSIS 3.11 per-user x64 installer**, using a separate installer script and build path from the legacy app's Tauri-generated NSIS installer.

Reasons:

- The native project is already an unpackaged `WinExe`; NSIS can wrap its published output without changing the app host or assigning package identity.
- NSIS is already part of the repository's Windows packaging stack for the separate HTML/Tauri app. Reusing it avoids another packaging dependency and its additional licensing/build-tool evaluation. The NSIS build remains a separate `.nsi` source, package directory and workflow path. [NSIS command-line and installer documentation](https://nsis.sourceforge.io/Docs/Chapter3.html)
- A non-elevated per-user install fits the app's existing `%LOCALAPPDATA%\RimV` data root and avoids an unnecessary UAC prompt. NSIS writes uninstall registration under HKCU and shortcuts to the current user's Start menu.
- The installer writes application files only under the selected install folder and current-user shortcuts/uninstall registration. It has no deletion rule for `%LOCALAPPDATA%\RimV`, so recordings/models/preferences are preserved by design. The app's named mutex makes setup/uninstall stop and ask the user to close RimV before changing loaded files.
- The package uses self-contained .NET and Windows App SDK deployment plus app-local VC++ runtime DLLs, avoiding separate runtime-download or prerequisite installer steps. Microsoft lists self-contained deployment as the option that removes the Windows App SDK runtime prerequisite for unpackaged direct-download apps, with a larger app folder as the tradeoff. [Microsoft unpackaged WinUI deployment](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/unpackage-winui-app)
- MSIX was not selected: this application is already explicitly configured for unpackaged hosting, and direct distribution requires package identity/signing and a different deployment model. WiX was not added because its MSI authoring and repair model are not needed for this per-user desktop app.
- The existing Tauri/NSIS HTML installer is a separate product path and remains unchanged. No current native installer exists to migrate.

Tradeoffs and limits: NSIS provides a conventional wizard, per-user uninstall registration and scripted replacement behavior, but no Store identity or automatic updates. The user must download/run a newer installer to update. The package is unsigned unless a certificate-backed signing stage is configured; unsigned artifacts can receive Windows trust warnings. This CI path does not publish installers.

## Windows build and native-library risks

- This Mac cannot verify the x86_64-pc-windows-msvc toolchain, WinUI publish output, NSIS compiler behavior or the actual DLL dependency closure. These are hard gates in the Windows workflow and remain unverified until it runs.
- `sherpa-onnx-sys` downloads a versioned upstream Windows runtime archive and stages its dynamic libraries, but the repository build script does not hash-verify that archive. The payload manifest hashes staged files after acquisition; that does not authenticate the upstream source. Add pinned upstream archive hashes before treating native dependency acquisition as supply-chain verified.
- Self-contained .NET/Windows App SDK plus app-local VC runtime avoids machine prerequisites at the cost of a larger installer and a wider bundled runtime inventory. The `dumpbin /DEPENDENTS` step can find direct unresolved imports, but it does not replace first-launch testing on a clean Windows image.
- The package is x64 only despite the WinUI project also declaring ARM64. ARM64 support needs a separate Rust/ASR runtime build and package validation.
- No Authenticode certificate is available to this workflow. Installer trust prompts are expected until signing is configured and validated.

## Production package design

`scripts/windows/build-native-package.ps1` uses the canonical workspace version, Cargo.lock, Rust 1.98.1, .NET SDK 10.0.105, Windows App SDK 2.0.1 and NSIS 3.11. It targets x86_64-pc-windows-msvc / `win-x64`, builds `rimv-ffi`, publishes the WinUI app as self-contained, and stages files only under `target/windows-native-package/`. It does not mutate the legacy Tauri runtime staging directory.

The payload contains the WinUI app/resources, the shared Rust C ABI library and dependent x64 native runtime DLLs, the .NET/Windows App SDK self-contained runtime, the app-local VC++ runtime, icon and license. Models are not embedded; shared-core downloads happen only after the user operates the app. PDB files are removed. No test projects, mock services, development server, HTML frontend, source checkout or temporary build output is included.

The installer installs to `%LOCALAPPDATA%\Programs\RimV Native Windows`, adds a Start menu shortcut and presents an unchecked optional desktop shortcut. It uses the NSIS Modern UI welcome, components, destination, progress and completion pages, with an optional final-page launch. It adds no prerequisite downloads or custom promotional pages. The app and installer use the shared RimV `.ico`. Data under `%LOCALAPPDATA%\RimV` is outside the install folder and is never silently removed.

Build artifacts when run on Windows:

- `RimV-0.1.0-beta-windows-x64-setup.exe`
- `RimV-0.1.0-beta-windows-x64-manifest.json` (commit, ABI, runtime mode, native DLL inventory, per-file SHA-256)
- `RimV-0.1.0-beta-windows-x64-SHA256SUMS.txt`

No portable ZIP is generated.

## Separate packaging and lifecycle tests

`apps/windows-native.packaging-tests/Test-NativePackage.ps1` is outside the production project. Payload mode verifies manifest hashes and exact file inventory, excludes test/debug content, checks the GUI PE subsystem and x64 PE machine type, validates native DLL presence, and scans native imports for dependencies absent from the package/system. Installer mode verifies distribution checksums; installs the exact artifact to a path containing spaces; checks Start menu, shared Rust DLL and installed file hashes; smoke-launches the WinUI process; repeats installation; uninstalls; and confirms a recording-data sentinel is preserved.

`.github/workflows/windows-native-package.yml` is a manual/PR validation workflow, not a release workflow. Its package job runs Rust regression tests, .NET coordinator tests and the real FFI C# integration test, builds/statically inspects the package, then uploads a 14-day workflow artifact. A separate fresh Windows runner downloads that exact installer and runs installation/reinstall/uninstall checks. The workflow does not create a release or tag.

## Installer UX review

The install flow has one standard Windows wizard, product/version identification, one install destination, visible native install progress, and an optional unchecked desktop link. Start menu integration is always present; the completed page offers launch. It does not request elevation, request microphone permission during install, fetch models, run the app in a browser, or auto-start at login. AppMutex prevents in-place replacement while a capture app process is still open. Data is preserved with explicit manual deletion instructions in `docs/windows-native-installation.md`.

Heuristic findings/resolutions:

- **H3 User control and freedom — severity 2:** upgrades/uninstall must not abruptly replace a live capture process. `AppMutex` blocks those operations until the user closes RimV and retries.
- **H5 Error prevention — severity 2:** the common desktop shortcut is optional and unchecked; setup runs without elevation; x64-only packages declare architecture restrictions.
- **H9 Recoverable errors — severity 2:** NSIS reports installation errors; explicit messages explain how to close RimV when its mutex prevents install/uninstall. The setup shows detailed progress; automated lifecycle failures surface in the Actions log.
- **H8 Minimalist design — severity 1:** no custom splash, promotional screens, bundled model download or optional components page is added.

This is a source/configuration review. The real wizard, scaling, keyboard navigation and screen-reader behavior have not been observed on Windows; do not treat these resolutions as visual/a11y sign-off.

## Validation record

Passed on macOS development host:

- PowerShell parser syntax check for `build-native-package.ps1` and `Test-NativePackage.ps1`.
- YAML parse of `.github/workflows/windows-native-package.yml`.
- `git diff --check`.
- `make build` completed successfully after the Phase 3 Rust compile fixes.
- `cargo fmt --all -- --check` passed; `model-manager` integration tests passed (4/4); `speech-transcription --no-default-features` passed (22/22).
- Earlier in the same worktree, `make menu` built/opened the existing macOS menu app, and `make menu-test` returned `Native menu self-test: PASS`. The Phase 4 edits do not touch macOS UI code.

Not run / pending on Windows:

- Rust Windows x64 release build and exact native DLL inventory.
- WinUI self-contained Release publish, NSIS compile, installer, manifest and checksum generation. No installer artifact was generated on this Mac.
- Rust and .NET Windows regression workflow, including C# FFI loading of the Windows DLL.
- Fresh-runner install, first-run launch, repeated install, uninstallation and recording-sentinel check.
- Actual tray presence/menu, user-driven graceful shutdown, real upgrade from an older native package (none exists yet), Start-menu UX, signed publisher trust, Narrator/keyboard/scaling/theme and physical WASAPI capture.

The CI startup smoke test checks that the installed process remains alive for its observation interval, not tray visibility or graceful shutdown. The test harness force-stops that process so it can continue with reinstall/uninstall. Those UI lifecycle checks require an interactive Windows desktop. Same-build reinstall is reported as a reinstall/repair check, not proof of upgrade from a previous version.

## Skills and guidance applied

Project skills read and applied:

- `.agents/skills/clean-code/SKILL.md`
- `.agents/skills/windows-native/SKILL.md`
- `.agents/skills/windows-ui/SKILL.md`
- `.agents/skills/ci-release/SKILL.md`
- `.agents/skills/cross-platform/SKILL.md`
- `.agents/skills/rust-workspace/SKILL.md`
- `.agents/skills/api-contracts/SKILL.md`
- `.agents/skills/testing/SKILL.md`
- `.agents/skills/security-privacy/SKILL.md`
- `.agents/skills/ui-design/SKILL.md`
- `.agents/skills/usability-heuristics/SKILL.md`

Specialist/design guidance: `.agents/agents/devops-engineer.md`, `.agents/agents/windows-developer.md`, `.agents/agents/software-architect.md`, `.agents/agents/test-engineer.md`, `.agents/agents/ui-ux-designer.md`, `.agents/design/RIMV_DESIGN_SYSTEM.md`, `.agents/design/platforms/windows/WINDOWS_PLATFORM_GUIDELINES.md`, `.agents/design/platforms/windows/WINUI_IMPLEMENTATION_GUIDE.md`, `.agents/design/platforms/windows/WINDOWS_UI_REVIEW_CHECKLIST.md`, `.agents/ux/usability-heuristics/RIMV_USABILITY_HEURISTICS.md` and `.agents/ux/usability-heuristics/RIMV_FLOW_REVIEW_GUIDE.md`.

No dedicated Windows installer/packaging skill, standalone C# skill or standalone TDD skill exists in `.agents/skills/`. Packaging followed the Windows-native and CI/release skills; C# integration followed the existing WinUI project, API contract and Windows native guidance. No unavailable skill is claimed as used.

## Open gates

1. Run `.github/workflows/windows-native-package.yml` on Windows and correct actual toolchain/package failures.
2. Inspect the generated manifest and dependency scan against the produced x64 payload. The expected dynamic DLL list is not asserted from an unobserved Windows build; generation fails if Rust FFI and sherpa/ONNX outputs are missing.
3. Perform interactive Windows install, tray, graceful shutdown, theme/accessibility/scaling, and physical capture checks before calling the installer validated.
4. Add Authenticode signing when a valid trusted signing certificate is provisioned; retain accurate unsigned status until then.
5. Perform a true older-to-newer native installer upgrade test after there is a previous native Windows installer. The legacy Tauri/HTML installer is not a compatible predecessor.

No release or version tag was created.
