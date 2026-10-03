# RimV Native Windows distribution audit

## Current release configuration

`apps/windows-native/RimV.Windows.csproj` targets `net10.0-windows10.0.26100.0`, declares `SupportedOSPlatformVersion` 10.0.19041.0, and sets `WindowsPackageType=None`. The package build then publishes `win-x64` with `--self-contained true`, `WindowsAppSDKSelfContained=true`, trimming and single-file disabled, and ReadyToRun disabled. It copies Rust-built native DLLs and the app-local x64 Visual C++ runtime into the publish directory. A signed sparse identity MSIX is registered next to the NSIS app; the MSIX is not the app payload. This is a self-contained .NET and self-contained Windows App SDK deployment with package identity via external location, not a dynamically dependent Windows App SDK deployment.

The current deployment bundles the native and managed dependencies so the app can launch offline without a separately installed .NET 10 or Windows App SDK runtime. The cost is a larger installer and duplicated runtime files across apps. Framework-dependent Windows App SDK deployments use a machine-installed framework package and require explicit runtime installation for unpackaged or external-location apps. Microsoft documents these models in its [Windows App SDK deployment overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/deploy-overview) and [self-contained deployment guide](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/self-contained-deploy/deploy-self-contained-apps).

| Component | Distribution classification | Reason |
|---|---|---|
| RimV executable and managed app assembly | REQUIRED | Main WinUI host and coordinator. |
| Rust FFI, audio capture, inference and ONNX/sherpa DLLs | REQUIRED | The shared Rust engine owns capture, local transcription and model management. |
| WinUI / Windows App SDK runtime | REQUIRED in current deployment; DEPLOYMENT-STRATEGY DEPENDENT | Self-contained publish places it beside the app; a framework-dependent variant would require matching system runtime deployment. |
| .NET runtime | REQUIRED in current deployment; DEPLOYMENT-STRATEGY DEPENDENT | Self-contained .NET 10 is currently published; a framework-dependent variant would require the matching .NET runtime. |
| App-local VC runtime DLLs | REQUIRED by current native DLL linkage | May only be removed after dependency scanning and a clean-machine launch test demonstrate they are unnecessary. |
| App icon, flag assets, license | REQUIRED or feature/resource dependent | Brand, language UI, and license resources copied by the native project. |
| PDBs and intermediate build artifacts | REMOVABLE | Public build explicitly disables and rejects these files. |
| HTML/Tauri frontend and old Windows resources | REMOVABLE from the native distribution; preserved in the repository | They are not inputs to the WinUI project and are rejected by payload validation. |
| Parakeet and Whisper model weights | OPTIONAL | Downloaded only when selected in setup or Model Manager; never included in the base installer. |
| Sparse identity package | REQUIRED for Windows Native Speech; DEPLOYMENT-STRATEGY DEPENDENT | Current external-location identity grants the system AI capability; another identity-bearing package layout could replace it. |

## Build inputs and exclusions

- Rust release profile: `opt-level` is not set, so Cargo's release default is 3; `lto=thin`; `codegen-units=1`; `strip="symbols"`; `panic` is not set, so the Rust default is unwind. Cargo's [profile reference](https://doc.rust-lang.org/cargo/reference/profiles.html) documents these defaults.
- Windows engine features include Silero VAD, Parakeet, Whisper, and the Windows Native Speech bridge. Model weights are not linked or embedded. Whisper Base and Parakeet model data go under `%LOCALAPPDATA%\RimV\models` only when installed.
- .NET publish requests `DebugType=None` and `DebugSymbols=false`; the build also removes and then rejects PDB/intermediate files.
- Windows branding/flag assets come from `apps/windows/icons` and `apps/windows-native/Assets`. `resources/Assets.xcassets` is not copied.
- The publish directory is deleted and recreated for each package. The release payload validation rejects `.html`, `.js`, `.css`, Tauri/Vite/node_modules trees, model files, and macOS asset catalogs. It permits only WinUI's known `Microsoft.UI.Xaml/Assets/map.html` resource.
- Existing source-tree sizes measured on this checkout: Tauri `apps/windows/ui` 36 KiB; `apps/windows/runtime` is empty; Windows native flags 172 KiB; `apps/windows/icons` 8 KiB; macOS `resources/Assets.xcassets` 216 KiB. These are source-tree measurements, not package measurements.

## Why the old Tauri installer was smaller

`apps/windows/tauri.conf.json` uses `frontendDist: "ui"` and the NSIS `downloadBootstrapper` WebView2 mode. The checked-in Tauri frontend is about 36 KiB; the configured `apps/windows/runtime/` resource folder is empty. Tauri's installer mode adds 0 MB of WebView2 runtime payload and downloads/runs Microsoft's bootstrapper if the runtime needs installation. Tauri's [Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/#webview2-installation-options) describes this as the smallest online installer mode; Microsoft describes WebView2 Evergreen as a shared system runtime in its [distribution guidance](https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution). Therefore the prior approximately 5 MB number represents the app/installer payload reported for Tauri, with WebView2 provided by the machine or installed separately during setup. The native app instead carries .NET, WinUI/Windows App SDK, Rust inference DLLs, and app-local VC runtime. This comparison does not establish that RimV's own application logic grew by 55 MB.

## Alternatives considered

Exact installer and installed-size estimates for these variants are **not available** until they are built and measured on Windows. This checkout has no Windows publish output to use as a baseline, so size estimates would be speculative.

| Deployment | Prerequisites and compatibility | Installer/offline/update behavior | Reliability tradeoff |
|---|---|---|---|
| Current: self-contained .NET + self-contained Windows App SDK | Windows 10 build 19041+ remains the app floor; no separate .NET or WinAppSDK install | Larger app payload; install and launch work offline; app updates carry the same runtimes | Most predictable clean install; app must ship runtime security updates |
| Framework-dependent .NET + self-contained Windows App SDK | Requires compatible .NET 10 runtime; same Windows app floor | Smaller app payload on machines already provisioned; NSIS must check/install .NET; offline setup must include runtime | Smaller if prerequisite exists; more setup failure states and system prerequisites |
| Self-contained .NET + framework-dependent Windows App SDK | Requires the matching Windows App SDK framework runtime and bootstrapper initialization | Can share Windows App SDK binaries across apps; offline NSIS must deploy matching runtime packages | System servicing can update shared runtime; exact framework registration and servicing add setup complexity |
| Framework-dependent .NET + framework-dependent Windows App SDK | Requires both runtime families | Potentially smallest app payload when all prerequisites exist; offline installer must include/deploy both to preserve offline behavior | Most prerequisite-sensitive and least predictable on a clean machine |

No deployment change was selected solely for size. Trimming, single-file publishing, framework-dependent deployment, and removal of native DLLs remain unselected until the actual Windows build and clean-machine launch path are measured and validated.

## Measurement status

The package build now creates `RimV-<version>-windows-x64-size-audit.md`. Its JSON manifest records exact payload files/hashes, component totals, the installer byte count, the expanded payload total, the main executable, and every distributed file larger than 1 MiB. The Windows installer lifecycle test prints exact installer, installed-directory, and main executable sizes. The artifact test rejects debug files and accidental legacy/model/macOS payloads. CI adds the audit report to its job summary.

There is no `target/windows-native-package` output in the current macOS checkout, so these values have not been measured here:

- Before/after installer size and saved bytes/percent.
- Installed directory size, including NSIS-generated `Uninstall.exe`.
- Main executable, Rust FFI DLL, WinUI/.NET/native dependency category sizes.
- PDB/debug and greater-than-1-MiB distributed-file inventory from a real release payload.

The approximately 60 MB current size is the user's reported estimate, not a locally re-measured baseline. Existing publish configuration already disables debug symbols and ships neither model data nor the Tauri frontend. No size-only binary optimization was made without a Windows payload measurement. The package CI audit will produce an exact **after** measurement on its next Windows run; an exact **before** comparison still requires the previous 60 MB build artifact or a reproducible build of that earlier revision.
