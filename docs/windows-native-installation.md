# RimV Native Windows installation

## Package format and support

The native WinUI 3 application uses a standalone **NSIS 3.11 x64 installer**. Its script and build pipeline are separate from the existing Tauri/NSIS HTML Windows application; the legacy app configuration and runtime staging remain untouched. No portable ZIP is produced.

The installer runs per user without elevation and defaults to `%LOCALAPPDATA%\Programs\RimV Native Windows`. It adds a Start menu shortcut and offers an optional desktop shortcut, unchecked by default. It includes a normal welcome, destination, progress and completion flow, and can launch RimV from the final page. It does not download models, open a browser or launch a terminal.

The package targets Windows x64, minimum Windows 10 build 19041. The native project targets the Windows 10 SDK 26100 for compile-time APIs but keeps `SupportedOSPlatformVersion` at 19041; the installer manifest uses the same minimum. The release script publishes self-contained .NET 10 and self-contained Windows App SDK 2.3.2 Experimental A, plus `rimv_core_ffi.dll`, inference DLL dependencies, app-local x64 Visual C++ runtime DLLs, RimV icon, flag assets and license. This is not dynamically dependent Windows App SDK deployment: the app carries the WinUI runtime files beside its executable. The sparse MSIX grants package identity and capabilities; it does not contain the app payload.

The installer also registers a signed sparse identity package beside the existing NSIS install. This preserves RimV's executable location and installer while granting the WinUI process package identity and the `systemAIModels` capability. The package manifest keeps Windows 10 build 19041 as its minimum and declares `MaxVersionTested` 10.0.26226.0 as required by Microsoft's Windows AI guidance. The Win32 application manifest's `msix` identity metadata must match the sparse package name, publisher and application ID. Windows Native Speech is only exposed on Windows 11 24H2+ when the API, system capability and speech model readiness checks pass; Windows 10 installations continue to use the existing local engines.

Windows AI Speech is experimental. RimV pins the speech adapter to the 2.3 Experimental A API surface (`Microsoft.WindowsAppSDK` 2.3.2-experimentalA) behind `INativeSpeechBridge`. Microsoft's 2.5 Experimental release changes model creation to factory/options APIs, so moving to that release requires updating this adapter and revalidating the streaming event contract. The native Windows installer build requires `RIMV_IDENTITY_CERTIFICATE` and `RIMV_IDENTITY_CERTIFICATE_PASSWORD` for a certificate trusted by target computers. Development certificates must be explicitly trusted on the test machine. The ordinary NSIS installer remains Authenticode-unsigned unless configured separately.

## Optional transcription models

The NSIS Components page offers two optional model downloads. Both are unchecked by default, including during `/S` silent installation:

- Parakeet TDT 0.6B v3 INT8: approximately 487 MB compressed download (487,170,055 bytes).
- Whisper Base: approximately 148 MB download (147,951,465 bytes); this is the existing catalog's balanced real-time Whisper option.
- Either local ASR option also installs the shared Silero VAD prerequisite through Model Manager (about 0.63 MB); the prerequisite is downloaded only when an ASR option is selected and is shared if both are selected.

The release script derives those labels and manifest sizes from the shared Rust catalog. Both NSIS sections are optional and unchecked by default, including `/S` silent installation. Admins can opt into them through NSIS `/COMPONENTS=SecParakeet,SecWhisperBase`. The installer launches the installed RimV executable in a one-shot model-install mode. That mode calls the same Rust FFI and `model-manager` code used by Model Manager; NSIS contains no model URLs, download implementation or model payload. Whisper Base uses the catalog's existing size and SHA-256 verification. Parakeet now pins its archive size and SHA-256, downloads to a `.part` file, verifies before extraction, extracts to a staging directory, validates required files, then atomically publishes the model directory. A failed optional download leaves the app installed and reports the failed model; it can be retried from Model Manager. Model files live under `%LOCALAPPDATA%\RimV\models`, outside the app directory, so upgrades and uninstall preserve them. The setup selection does not change the saved engine preference: fresh supported Windows systems still default to Native Speech, while unsupported systems choose an already installed local backend when one exists.

Windows x64 release builds include both the Parakeet and Whisper backends. The prior Windows target excluded the Whisper inference feature even though its catalog and coordinator supported Whisper, which made those catalog rows unusable. Windows 10 build 19041 and later can use either local backend; Native Speech remains gated to supported Windows 11 systems.

The installer is unsigned unless a certificate-backed Authenticode step is configured. Windows may show an unknown-publisher or SmartScreen warning for an unsigned installer. Do not describe an unsigned artifact as publisher-verified.

## Install, update and uninstall

1. Run `RimV-<version>-windows-x64-setup.exe` and follow setup. Setup first checks whether RimV's identity-package signing certificate is already trusted. If not, it asks before adding the certificate to this PC's Trusted People store; Windows may request administrator approval. The certificate is not added to Trusted Root. Decline to install with Parakeet/Whisper only. Silent installs register Native Speech only when the certificate is already trusted. The matching public certificate is also published as `RimV-<version>-identity-publisher.cer` and included inside the installer for later manual registration.
2. Start RimV from Start > RimV > RimV Native Windows. Closing the popup does not quit the app.
3. To update, quit RimV from its tray menu, then run the newer installer. The stable per-user installation identity replaces the previous app files. Setup stops and asks for RimV to be closed if it is still running.
4. To remove RimV, use Windows Settings > Apps > Installed apps or `Uninstall.exe` in the installation folder. Uninstall removes app files and shortcuts but preserves `%LOCALAPPDATA%\RimV`, including recordings, models and preferences.

The installer never deletes user data. To delete personal data, uninstall first, then review and manually remove `%LOCALAPPDATA%\RimV`. That folder can be shared with other RimV Windows builds; inspect it before deletion.

## Diagnostics

The native Windows app writes rotating diagnostic logs under `%LOCALAPPDATA%\RimV\logs`:

- `windows-native.log` records app launch, single-instance activation, tray setup, shared-core operation names, event types, errors and shutdown.
- `rust-engine.log` records Rust engine tracing output when available.

The Windows host log rotates at 4 MiB with one previous file retained. The Rust engine log is moved to `.1` at the next app launch if it has reached 4 MiB. Logs exclude transcript text and audio buffers. File paths containing the current user's profile are redacted in the Windows host log. Review both files before sharing them because Rust diagnostics and exception details can still include local system information.

There has not been a previous native WinUI installer. Upgrade from the HTML/Tauri installer is not applicable: the apps have separate installer identities and application folders. A later native installer first runs the previous native uninstaller, then installs the new payload.

## Reproducible package build

Run on a clean Windows x64 environment with Rust 1.98.1 (MSVC target), .NET SDK 10.0.105, Visual Studio x64 C++ tools/redistributable, Windows SDK MakeAppx/SignTool, NSIS 3.11, and a trusted identity-package signing certificate configured in the two `RIMV_IDENTITY_CERTIFICATE*` environment variables:

```powershell
./scripts/windows/build-native-package.ps1
```

The script reads the product version from the root `Cargo.toml`, builds the Rust FFI and WinUI frontend in Release mode, stages only publish output plus runtime dependencies, and creates:

- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-setup.exe`
- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-manifest.json`
- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-SHA256SUMS.txt`

The manifest records the build commit, FFI API version, OS floor, runtime deployment choices, native DLL list, file inventory and SHA-256 hashes. Cargo uses `--locked`; this build does not mutate `apps/windows/runtime` or the legacy Tauri bundle.

The package job also writes `RimV-<version>-windows-x64-size-audit.md` and adds size totals and every distributed file over 1 MiB to the JSON manifest. It reports the compressed installer, expanded publish payload, main executable, component category totals, and largest files. The fresh-runner installer test reports the actual installed directory size including NSIS's generated uninstaller. The audit fails if debug/intermediate files, macOS asset catalogs, model data, or legacy HTML/Tauri frontend files enter the base payload. Model downloads are separate and are not part of these totals.

The former Tauri configuration uses `webviewInstallMode.type = downloadBootstrapper`; it does not embed the WebView2 runtime. Its checked-in HTML/CSS/JavaScript frontend currently totals roughly 36 KiB, and the configured `apps/windows/runtime` resource directory is empty. Thus the earlier approximately 5 MB installer represented RimV/Tauri's app payload while relying on an external/system WebView2 runtime (or downloading that runtime during setup), not an equivalent fully self-contained UI runtime. The current native release deliberately bundles .NET, WinUI/Windows App SDK, the Rust inference engine and app-local VC runtime so installation works offline without a user repairing missing runtimes.

Other deployment choices are possible, but their size savings are not measured in this checkout because no Windows publish output or installer is available here. Framework-dependent .NET could reduce app payload if .NET 10 is already installed, at the cost of a prerequisite or an offline-capable runtime installer. Framework-dependent Windows App SDK could share a machine-wide runtime, but NSIS would need to install the exact SDK runtime and maintain its registration/servicing lifecycle. Making both frameworks machine prerequisites could make the downloaded app smallest while making first install and offline installation less predictable. The current self-contained deployment favors predictable offline installation. A single-file or trimmed variant is not enabled; WinUI reflection/trimming and the experimental Speech API need Windows build and launch validation before changing those settings.

Release size measurements are generated by the Windows package CI job. This macOS checkout currently has no `target/windows-native-package` payload or installer, so installer, installed-directory, EXE, DLL category, PDB, and >1 MiB file sizes cannot be reported as measured values here. Existing package configuration already disables debug symbols, removes PDB files, starts from a clean publish directory, and copies no web or macOS asset trees. No size-only runtime optimization was applied without a real Windows before/after measurement.

Run payload checks from a Visual Studio Developer PowerShell with `dumpbin.exe` available:

```powershell
$manifest = Get-ChildItem target/windows-native-package/artifacts/*-manifest.json | Select-Object -First 1
./apps/windows-native.packaging-tests/Test-NativePackage.ps1 `
  -Mode Payload `
  -ManifestPath $manifest.FullName `
  -PayloadDirectory (Join-Path $PWD 'target/windows-native-package/payload')
```

The `RimV CI` workflow calls the native Windows validation jobs for relevant pull requests and main-branch pushes. Pull requests run fast code checks. Main-branch pushes and manual CI dispatches build and test the package on one Windows runner, upload that exact installer as a short-lived artifact of the CI run, then install it on a separate fresh Windows runner into a path containing spaces. The test checks startup, same-build reinstall, uninstall and preservation of a recording-data sentinel. CI does not create a GitHub Release or tag. A prerelease consumes the validated artifact from its exact commit after automated validation passes. Manual interactive Windows checks remain recommended, but do not block the release workflow.

To run the Windows AI Speech provider fixture E2E on a registered Windows 11 24H2+ machine with the optional speech model already installed, start the installed `RimV.Windows.exe --native-speech-e2e` with the RimV checkout as its working directory (or set `RIMV_REPO_ROOT` to the checkout). The test refuses to download a model, pushes `resources/audio/audio1-16k-mono.wav` incrementally through `SpeechAudioProvider`, forces the Native provider without Parakeet/Whisper fallback, and writes timing, partial/final counts, expected/actual text, WER and CER to `target/windows-native-speech-e2e.txt`. This validates the Windows speech provider and transcript mapping; WASAPI-to-provider forwarding remains part of the normal app path and is not exercised by this fixture runner.

## Validation still required on an interactive Windows desktop

The automated startup smoke test proves that the installed WinUI process remains running during its observation window. It does not prove tray visibility or graceful user-driven shutdown. Before distribution, manually verify the tray icon/menu, Quit behavior, install/update/uninstall, light/dark/high-contrast rendering, keyboard/Narrator use, display scaling and microphone/system-audio capture on supported hardware. Record Windows version, x64 architecture, installer version and signing status. A successful package build alone is not installer release approval.
