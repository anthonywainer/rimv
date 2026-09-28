# RimV Native Windows installation

## Package format and support

The native WinUI 3 application uses a standalone **NSIS 3.11 x64 installer**. Its script and build pipeline are separate from the existing Tauri/NSIS HTML Windows application; the legacy app configuration and runtime staging remain untouched. No portable ZIP is produced.

The installer runs per user without elevation and defaults to `%LOCALAPPDATA%\Programs\RimV Native Windows`. It adds a Start menu shortcut and offers an optional desktop shortcut, unchecked by default. It includes a normal welcome, destination, progress and completion flow, and can launch RimV from the final page. It does not download models, open a browser or launch a terminal.

The package targets Windows x64, minimum Windows 10 build 19041. It contains the self-contained .NET 10 app and Windows App SDK 2.0.1 runtime, `rimv_core_ffi.dll` and its Rust inference DLL dependencies, app-local x64 Visual C++ runtime DLLs, RimV icon and license. Speech models are downloaded later by the shared Rust model manager.

The installer is unsigned unless a certificate-backed Authenticode step is configured. Windows may show an unknown-publisher or SmartScreen warning for an unsigned installer. Do not describe an unsigned artifact as publisher-verified.

## Install, update and uninstall

1. Run `RimV-<version>-windows-x64-setup.exe` and follow setup.
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

Run on a clean Windows x64 environment with Rust 1.98.1 (MSVC target), .NET SDK 10.0.105, Visual Studio x64 C++ tools/redistributable, and NSIS 3.11:

```powershell
./scripts/windows/build-native-package.ps1
```

The script reads the product version from the root `Cargo.toml`, builds the Rust FFI and WinUI frontend in Release mode, stages only publish output plus runtime dependencies, and creates:

- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-setup.exe`
- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-manifest.json`
- `target/windows-native-package/artifacts/RimV-<version>-windows-x64-SHA256SUMS.txt`

The manifest records the build commit, FFI API version, OS floor, runtime deployment choices, native DLL list, file inventory and SHA-256 hashes. Cargo uses `--locked`; this build does not mutate `apps/windows/runtime` or the legacy Tauri bundle.

Run payload checks from a Visual Studio Developer PowerShell with `dumpbin.exe` available:

```powershell
$manifest = Get-ChildItem target/windows-native-package/artifacts/*-manifest.json | Select-Object -First 1
./apps/windows-native.packaging-tests/Test-NativePackage.ps1 `
  -Mode Payload `
  -ManifestPath $manifest.FullName `
  -PayloadDirectory (Join-Path $PWD 'target/windows-native-package/payload')
```

The `RimV CI` workflow calls the native Windows validation jobs for relevant pull requests and main-branch pushes. Pull requests run fast code checks. Main-branch pushes and manual CI dispatches build and test the package on one Windows runner, upload that exact installer as a short-lived artifact of the CI run, then install it on a separate fresh Windows runner into a path containing spaces. The test checks startup, same-build reinstall, uninstall and preservation of a recording-data sentinel. CI does not create a GitHub Release or tag. A prerelease consumes the validated artifact from its exact commit after automated validation passes. Manual interactive Windows checks remain recommended, but do not block the release workflow.

## Validation still required on an interactive Windows desktop

The automated startup smoke test proves that the installed WinUI process remains running during its observation window. It does not prove tray visibility or graceful user-driven shutdown. Before distribution, manually verify the tray icon/menu, Quit behavior, install/update/uninstall, light/dark/high-contrast rendering, keyboard/Narrator use, display scaling and microphone/system-audio capture on supported hardware. Record Windows version, x64 architecture, installer version and signing status. A successful package build alone is not installer release approval.
