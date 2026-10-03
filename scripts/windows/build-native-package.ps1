[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

function Get-PeMachine([string] $path) {
    $stream = [IO.File]::OpenRead($path)
    try {
        $reader = [IO.BinaryReader]::new($stream)
        $stream.Position = 0x3c
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) { return 0 }
        return $reader.ReadUInt16()
    }
    finally { $stream.Dispose() }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$projectFile = Join-Path $repoRoot 'apps\windows-native\RimV.Windows.csproj'
$installerScript = Join-Path $repoRoot 'apps\windows-native\packaging\rimv-native.nsi'
$identityManifest = Join-Path $repoRoot 'apps\windows-native\packaging\identity\Package.appxmanifest'
$identityRegistrationScript = Join-Path $repoRoot 'apps\windows-native\packaging\Register-Identity.ps1'
$cargoManifest = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw
$versionMatch = [regex]::Match($cargoManifest, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)"\s*$')
if (-not $versionMatch.Success) { throw 'Could not read the canonical semantic version from Cargo.toml.' }
$appVersion = $versionMatch.Groups['version'].Value
$numericMatch = [regex]::Match($appVersion, '^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)(?:-[0-9A-Za-z.-]+)?$')
if (-not $numericMatch.Success) { throw "Unsupported application version: $appVersion" }
$numericVersion = '{0}.{1}.{2}.0' -f $numericMatch.Groups['major'].Value, $numericMatch.Groups['minor'].Value, $numericMatch.Groups['patch'].Value

if ($env:RIMV_IDENTITY_CERTIFICATE_BASE64) {
    $certificatePath = Join-Path $env:RUNNER_TEMP 'RimV-identity-publisher.pfx'
    [IO.File]::WriteAllBytes($certificatePath, [Convert]::FromBase64String($env:RIMV_IDENTITY_CERTIFICATE_BASE64))
    $env:RIMV_IDENTITY_CERTIFICATE = $certificatePath
}

if (-not $env:RIMV_IDENTITY_CERTIFICATE -or -not (Test-Path -LiteralPath $env:RIMV_IDENTITY_CERTIFICATE -PathType Leaf)) {
    throw 'Set RIMV_IDENTITY_CERTIFICATE to the trusted publisher PFX required to sign the Windows AI identity package.'
}
if (-not $env:RIMV_IDENTITY_CERTIFICATE_PASSWORD) {
    throw 'Set RIMV_IDENTITY_CERTIFICATE_PASSWORD for the trusted publisher PFX.'
}

if (-not $IsWindows) { throw 'The native Windows release package must be built on Windows.' }
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw 'The v0.1 native installer target is Windows x64 only.'
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Install the pinned Rust toolchain before packaging.' }
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) { throw 'Install the .NET 10 SDK before packaging.' }

$projectXml = [xml](Get-Content -LiteralPath $projectFile -Raw)
$projectVersion = $projectXml.Project.PropertyGroup.Version | Select-Object -First 1
if ($projectVersion -ne $appVersion) {
    throw "Native Windows project version '$projectVersion' differs from workspace version '$appVersion'. Update both version sources before packaging."
}
$windowsAppSdkVersion = $projectXml.Project.ItemGroup.PackageReference |
    Where-Object { $_.Include -eq 'Microsoft.WindowsAppSDK' } |
    Select-Object -ExpandProperty Version -First 1
if (-not $windowsAppSdkVersion) { throw 'Microsoft.WindowsAppSDK package version is missing from the native app project.' }
$modelManagerSource = Get-Content -LiteralPath (Join-Path $repoRoot 'crates\model-manager\src\lib.rs') -Raw
$parakeetSizeMatch = [regex]::Match($modelManagerSource, '(?m)^const PARAKEET_ARCHIVE_SIZE: u64 = (?<size>[\d_]+);')
$whisperBaseSizeMatch = [regex]::Match($modelManagerSource, 'legacy_whisper\("base",\s*"Base",\s*"ggml-base\.bin",\s*(?<size>[\d_]+)')
if (-not $parakeetSizeMatch.Success -or -not $whisperBaseSizeMatch.Success) {
    throw 'Could not read optional model download sizes from the shared model catalog.'
}
$parakeetSizeBytes = [long]($parakeetSizeMatch.Groups['size'].Value.Replace('_', ''))
$whisperBaseSizeBytes = [long]($whisperBaseSizeMatch.Groups['size'].Value.Replace('_', ''))
$parakeetSizeMb = [long][Math]::Round($parakeetSizeBytes / 1e6, [MidpointRounding]::AwayFromZero)
$whisperBaseSizeMb = [long][Math]::Round($whisperBaseSizeBytes / 1e6, [MidpointRounding]::AwayFromZero)

$workRoot = Join-Path $repoRoot 'target\windows-native-package'
$payloadDirectory = Join-Path $workRoot 'payload'
$cargoTargetDirectory = Join-Path $workRoot 'cargo-target'
$artifactDirectory = Join-Path $workRoot 'artifacts'
if (Test-Path -LiteralPath $workRoot) { Remove-Item -LiteralPath $workRoot -Recurse -Force }
New-Item -ItemType Directory -Path $payloadDirectory, $artifactDirectory -Force | Out-Null

$previousCargoTarget = $env:CARGO_TARGET_DIR
try {
    $env:CARGO_TARGET_DIR = $cargoTargetDirectory
    Push-Location $repoRoot
    try {
        cargo build --locked --release --target x86_64-pc-windows-msvc -p rimv-ffi
        if ($LASTEXITCODE -ne 0) { throw 'The shared Rust core failed to build for Windows x64 MSVC.' }
    }
    finally { Pop-Location }

    $rustReleaseDirectory = Join-Path $cargoTargetDirectory 'x86_64-pc-windows-msvc\release'
    $rustDlls = @(Get-ChildItem -LiteralPath $rustReleaseDirectory -Filter '*.dll' -File | Sort-Object Name)
    if (-not ($rustDlls.Name -contains 'rimv_core_ffi.dll')) {
        throw 'The Windows shared-core build did not produce rimv_core_ffi.dll.'
    }
    if (-not ($rustDlls.Name | Where-Object { $_ -match '(?i)sherpa|onnxruntime' })) {
        throw 'No sherpa-onnx/ONNX Runtime DLLs were staged by the shared-core build.'
    }

    dotnet restore $projectFile `
        --runtime win-x64 `
        -p:Platform=x64 `
        -p:SelfContained=true `
        -p:WindowsAppSDKSelfContained=true `
        -p:EnableMsixTooling=true `
        -p:BuildRimvCore=false `
        -p:NuGetAudit=false
    if ($LASTEXITCODE -ne 0) { throw 'The locked native WinUI restore failed.' }

    dotnet publish $projectFile `
        --configuration Release `
        --framework net10.0-windows10.0.26100.0 `
        --runtime win-x64 `
        --self-contained true `
        --no-restore `
        --output $payloadDirectory `
        -p:Platform=x64 `
        -p:BuildRimvCore=false `
        -p:WindowsAppSDKSelfContained=true `
        -p:EnableMsixTooling=true `
        -p:PublishTrimmed=false `
        -p:PublishSingleFile=false `
        -p:PublishReadyToRun=false `
        -p:Version=$appVersion `
        -p:InformationalVersion=$appVersion `
        -p:IncludeSourceRevisionInInformationalVersion=false `
        -p:NuGetAudit=false `
        -p:DebugType=None `
        -p:DebugSymbols=false
    if ($LASTEXITCODE -ne 0) { throw 'The native WinUI 3 self-contained publish failed.' }

    foreach ($dll in $rustDlls) {
        Copy-Item -LiteralPath $dll.FullName -Destination $payloadDirectory -Force
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) { throw 'Visual Studio Installer vswhere.exe is required to locate the x64 VC runtime.' }
    $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $visualStudio) { throw 'The x64 MSVC toolchain/redistributable is missing.' }
    $redistRoot = Join-Path $visualStudio 'VC\Redist\MSVC'
    $redistBases = [Collections.Generic.List[string]]::new()
    if ($env:VCToolsRedistDir -and (Test-Path -LiteralPath $env:VCToolsRedistDir -PathType Container)) {
        $redistBases.Add($env:VCToolsRedistDir)
    }
    if (Test-Path -LiteralPath $redistRoot -PathType Container) {
        foreach ($versionDirectory in Get-ChildItem -LiteralPath $redistRoot -Directory | Sort-Object Name -Descending) {
            $redistBases.Add($versionDirectory.FullName)
        }
    }
    $crtCandidates = foreach ($redistBase in $redistBases) {
        $x64Directory = Join-Path $redistBase 'x64'
        if (Test-Path -LiteralPath $x64Directory -PathType Container) {
            Get-ChildItem -LiteralPath $x64Directory -Directory -Filter 'Microsoft.VC*.CRT'
        }
    }
    $crtDirectory = $null
    foreach ($candidate in $crtCandidates | Sort-Object Name -Descending) {
        $candidateDlls = @(Get-ChildItem -LiteralPath $candidate.FullName -Filter '*.dll' -File)
        if (($candidateDlls.Name -contains 'vcruntime140.dll') -and ($candidateDlls.Name -contains 'msvcp140.dll')) {
            $crtDirectory = $candidate
            break
        }
    }
    if (-not $crtDirectory) { throw "No x64 Microsoft Visual C++ runtime directory containing vcruntime140.dll and msvcp140.dll was found under $redistRoot or VCToolsRedistDir." }
    $crtDlls = @(Get-ChildItem -LiteralPath $crtDirectory.FullName -Filter '*.dll' -File)
    foreach ($requiredCrt in @('vcruntime140.dll', 'msvcp140.dll')) {
        if (-not ($crtDlls.Name -contains $requiredCrt)) { throw "The VC++ runtime is missing $requiredCrt." }
    }
    foreach ($dll in $crtDlls) { Copy-Item -LiteralPath $dll.FullName -Destination $payloadDirectory -Force }

    Copy-Item -LiteralPath $identityRegistrationScript -Destination $payloadDirectory
    $identityDirectory = Join-Path $workRoot 'identity-package'
    New-Item -ItemType Directory -Path $identityDirectory -Force | Out-Null
    $identityManifestText = Get-Content -LiteralPath $identityManifest -Raw
    $identityManifestText = $identityManifestText.Replace('__PACKAGE_VERSION__', $numericVersion)
    Set-Content -LiteralPath (Join-Path $identityDirectory 'Package.appxmanifest') -Value $identityManifestText -Encoding utf8
    $windowsKitBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $makeAppx = Get-ChildItem -LiteralPath $windowsKitBin -Filter MakeAppx.exe -File -Recurse |
        Where-Object { $_.Directory.Name -eq 'x64' } | Sort-Object FullName -Descending | Select-Object -First 1
    $signTool = Get-ChildItem -LiteralPath $windowsKitBin -Filter SignTool.exe -File -Recurse |
        Where-Object { $_.Directory.Name -eq 'x64' } | Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $makeAppx -or -not $signTool) { throw 'Windows SDK MakeAppx.exe and SignTool.exe are required to create the signed identity package.' }
    $identityPackage = Join-Path $payloadDirectory 'RimV.Identity.msix'
    & $makeAppx.FullName pack /o /d $identityDirectory /nv /p $identityPackage
    if ($LASTEXITCODE -ne 0) { throw 'MakeAppx failed to create RimV.Identity.msix.' }
    & $signTool.FullName sign /fd SHA256 /a /f $env:RIMV_IDENTITY_CERTIFICATE /p $env:RIMV_IDENTITY_CERTIFICATE_PASSWORD $identityPackage
    if ($LASTEXITCODE -ne 0) { throw 'SignTool failed to sign RimV.Identity.msix with the trusted publisher certificate.' }
    $certificatePassword = ConvertTo-SecureString $env:RIMV_IDENTITY_CERTIFICATE_PASSWORD -AsPlainText -Force
    $publisherCertificate = Get-PfxCertificate -FilePath $env:RIMV_IDENTITY_CERTIFICATE -Password $certificatePassword
    if ($publisherCertificate.Subject -ne 'CN=RimV') {
        throw "The identity certificate subject '$($publisherCertificate.Subject)' does not match Package.appxmanifest Publisher='CN=RimV'."
    }
    $publisherCertificatePath = Join-Path $artifactDirectory "RimV-$appVersion-identity-publisher.cer"
    Export-Certificate -Cert $publisherCertificate -FilePath $publisherCertificatePath | Out-Null

    $icon = Join-Path $repoRoot 'apps\windows\icons\icon.ico'
    $publishedIcon = Join-Path $payloadDirectory 'Assets\rimv.ico'
    if (-not (Test-Path -LiteralPath $icon)) { throw "The RimV application icon is missing: $icon" }
    if (-not (Test-Path -LiteralPath $publishedIcon)) {
        New-Item -ItemType Directory -Path (Split-Path $publishedIcon) -Force | Out-Null
        Copy-Item -LiteralPath $icon -Destination $publishedIcon
    }
    $licenseDirectory = Join-Path $payloadDirectory 'licenses'
    New-Item -ItemType Directory -Path $licenseDirectory -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $licenseDirectory 'RimV-LICENSE.txt')

    Get-ChildItem -LiteralPath $payloadDirectory -Filter '*.pdb' -File -Recurse | Remove-Item -Force
    $debugArtifacts = @(Get-ChildItem -LiteralPath $payloadDirectory -File -Recurse | Where-Object {
        $_.Extension -match '(?i)^\.(pdb|ilk|exp|iobj|ipdb)$'
    })
    if ($debugArtifacts.Count -gt 0) {
        throw "Debug/intermediate artifacts remain in the public payload: $($debugArtifacts.FullName -join ', ')"
    }
    $payloadPaths = @(Get-ChildItem -LiteralPath $payloadDirectory -File -Recurse | ForEach-Object {
        [IO.Path]::GetRelativePath($payloadDirectory, $_.FullName).Replace('\', '/')
    })
    $unexpectedLegacy = @($payloadPaths | Where-Object {
        ($_ -match '(?i)(^|/)(tauri|node_modules|vite)(/|$)|\.(html|js|css)$') -and
        $_ -ne 'Microsoft.UI.Xaml/Assets/map.html'
    })
    if ($unexpectedLegacy.Count -gt 0) {
        throw "Legacy frontend files are not allowed in the native Windows payload: $($unexpectedLegacy -join ', ')"
    }
    if ($payloadPaths | Where-Object { $_ -match '(?i)(Assets\.xcassets|\.onnx$|\.ggml$|\.tar\.bz2$)' }) {
        throw 'Model data or macOS asset catalogs must not be embedded in the base Windows installer.'
    }
    $exe = Join-Path $payloadDirectory 'RimV.Windows.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'Publish output does not contain RimV.Windows.exe.' }
    if (-not (Test-Path -LiteralPath (Join-Path $payloadDirectory 'rimv_core_ffi.dll'))) {
        throw 'Publish payload is missing the shared Rust FFI DLL.'
    }

    $nativeDllNames = @(Get-ChildItem -LiteralPath $payloadDirectory -Filter '*.dll' -File -Recurse |
        Where-Object { (Get-PeMachine $_.FullName) -eq 0x8664 } |
        ForEach-Object { [IO.Path]::GetRelativePath($payloadDirectory, $_.FullName).Replace('\', '/') } |
        Sort-Object -Unique)
    $payloadFiles = @(Get-ChildItem -LiteralPath $payloadDirectory -File -Recurse | Sort-Object FullName | ForEach-Object {
        [pscustomobject]@{
            path = [IO.Path]::GetRelativePath($payloadDirectory, $_.FullName).Replace('\', '/')
            size = $_.Length
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    })
    $commit = git -C $repoRoot rev-parse HEAD 2>$null
    if ($LASTEXITCODE -ne 0) { $commit = $env:GITHUB_SHA }
    $manifest = [ordered]@{
        schemaVersion = 1
        product = 'RimV Native Windows'
        version = $appVersion
        architecture = 'x64'
        minimumWindowsVersion = '10.0.19041.0'
        installerFormat = 'NSIS'
        installerVersion = '3.11'
        buildCommit = $commit
        sharedRustLibrary = 'rimv_core_ffi.dll'
        ffiApiVersion = 1
        deployment = [ordered]@{
            dotnet = 'self-contained .NET 10'
            windowsAppSdk = "self-contained Windows App SDK $windowsAppSdkVersion"
            visualCppRuntime = 'x64 app-local redistributable DLLs'
            modelAssets = 'downloaded on demand by the shared Rust model manager'
        }
        optionalModels = @(
            [ordered]@{ id = 'parakeet-tdt-0.6b-v3-int8'; sizeBytes = $parakeetSizeBytes; approximateDownloadMegabytes = $parakeetSizeMb },
            [ordered]@{ id = 'whisper-base'; sizeBytes = $whisperBaseSizeBytes; approximateDownloadMegabytes = $whisperBaseSizeMb }
        )
        signed = $false
        windowsAiIdentityPackage = 'RimV.Identity.msix (signed with trusted publisher certificate)'
        nativeRuntimeDlls = $nativeDllNames
        files = $payloadFiles
    }

    $manifestPath = Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-manifest.json"
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    $makensis = if ($env:MAKENSIS_EXE) { $env:MAKENSIS_EXE } else { Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
    if (-not (Test-Path -LiteralPath $makensis)) { throw "NSIS 3.11 compiler not found: $makensis" }
    $icon = Join-Path $repoRoot 'apps\windows\icons\icon.ico'
    & $makensis "/DPAYLOADDIR=$payloadDirectory" "/DARTIFACTDIR=$artifactDirectory" "/DAPP_VERSION=$appVersion" "/DNUMERIC_VERSION=$numericVersion" "/DPARAKEET_MODEL_SIZE_MB=$parakeetSizeMb" "/DWHISPER_BASE_MODEL_SIZE_MB=$whisperBaseSizeMb" "/DAPP_ICON=$icon" $installerScript
    if ($LASTEXITCODE -ne 0) { throw 'NSIS failed to build the native Windows installer.' }

    $installerPath = Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerPath)) { throw 'NSIS did not produce the expected installer.' }
    $categoryForPath = {
        param([string] $path)
        $name = [IO.Path]::GetFileName($path)
        if ($name -ieq 'RimV.Windows.exe') { return 'RimV executable' }
        if ($name -ieq 'rimv_core_ffi.dll') { return 'RimV Rust engine/library' }
        if ($path -match '(?i)^Assets/') { return 'Assets/resources' }
        if ($path -match '(?i)\.pdb$|\.(ilk|exp|iobj|ipdb)$') { return 'Debug symbols/intermediates' }
        if ($name -match '(?i)^(Microsoft\.UI|Microsoft\.Windows\.|Microsoft\.WindowsAppRuntime|Microsoft\.Graphics|WinRT\.Runtime|MRTCore|DWriteCore)') { return 'Windows App SDK / WinUI' }
        if ($name -match '(?i)^(coreclr|clrjit|hostfxr|hostpolicy|mscordaccore|System\.|Microsoft\.NETCore|Microsoft\.Extensions\.|RimV\.Windows(\.Application)?)') { return '.NET/runtime and RimV managed code' }
        if ($name -match '(?i)(sherpa|onnxruntime|vcruntime|msvcp)') { return 'Other native dependencies' }
        if ($path -match '(?i)(tauri|node_modules|vite)|\.(html|js|css)$') { return 'Legacy HTML/Tauri' }
        return 'Other files'
    }
    $categoryTotals = [ordered]@{
        'RimV executable' = [long]0
        'RimV Rust engine/library' = [long]0
        'Windows App SDK / WinUI' = [long]0
        '.NET/runtime and RimV managed code' = [long]0
        'Assets/resources' = [long]0
        'Debug symbols/intermediates' = [long]0
        'Legacy HTML/Tauri' = [long]0
        'Other native dependencies' = [long]0
        'Other files' = [long]0
    }
    foreach ($file in $payloadFiles) {
        $category = & $categoryForPath $file.path
        if (-not $categoryTotals.Contains($category)) { $categoryTotals[$category] = [long]0 }
        $categoryTotals[$category] += [long]$file.size
    }
    $payloadBytes = [long](($payloadFiles | Measure-Object -Property size -Sum).Sum)
    $largestFiles = @($payloadFiles | Where-Object { $_.size -gt 1MB } | Sort-Object size -Descending)
    $sizeAudit = [ordered]@{
        installerBytes = (Get-Item -LiteralPath $installerPath).Length
        expandedPayloadBytes = $payloadBytes
        installedDirectoryBytesBeforeUninstaller = $payloadBytes
        mainExecutableBytes = [long](($payloadFiles | Where-Object path -eq 'RimV.Windows.exe' | Select-Object -First 1).size)
        componentBytes = $categoryTotals
        filesOverOneMiB = $largestFiles
        caveats = @('Expanded payload is the sum of manifest files; installed directory measurement also includes NSIS-generated Uninstall.exe.', 'Optional transcription models are installed under %LOCALAPPDATA%\RimV\models and are excluded from this base payload.')
    }
    $manifest.sizeAudit = $sizeAudit
    $manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    $reportPath = Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-size-audit.md"
    $reportLines = [Collections.Generic.List[string]]::new()
    $reportLines.Add('# RimV Native Windows size audit')
    $reportLines.Add('')
    $reportLines.Add(('Installer: {0:N2} MB ({1:N2} MiB)' -f ($sizeAudit.installerBytes / 1e6), ($sizeAudit.installerBytes / 1MB)))
    $reportLines.Add(('Expanded payload: {0:N2} MB ({1:N2} MiB)' -f ($payloadBytes / 1e6), ($payloadBytes / 1MB)))
    $reportLines.Add(('Main executable: {0:N2} MB ({1:N2} MiB)' -f ($sizeAudit.mainExecutableBytes / 1e6), ($sizeAudit.mainExecutableBytes / 1MB)))
    $reportLines.Add('')
    $reportLines.Add('## Component totals (expanded payload)')
    foreach ($category in $categoryTotals.Keys) {
        $reportLines.Add(('- {0}: {1:N2} MB ({2:N2} MiB)' -f $category, ($categoryTotals[$category] / 1e6), ($categoryTotals[$category] / 1MB)))
    }
    $reportLines.Add('')
    $reportLines.Add('## Distributed files larger than 1 MiB')
    foreach ($file in $largestFiles) {
        $reportLines.Add(('- {0}: {1:N2} MB ({2:N2} MiB)' -f $file.path, ($file.size / 1e6), ($file.size / 1MB)))
    }
    $reportLines.Add('')
    $reportLines.Add('Optional model files are not part of the installer payload.')
    $reportLines | Set-Content -LiteralPath $reportPath -Encoding utf8
    $checksums = @($installerPath, $manifestPath, $publisherCertificatePath) | ForEach-Object {
        '{0}  {1}' -f (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant(), (Split-Path $_ -Leaf)
    }
    $checksums | Set-Content -LiteralPath (Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-SHA256SUMS.txt") -Encoding ascii
    Write-Host "Created the x64 installer, manifest and size audit under $artifactDirectory (unsigned unless Authenticode is configured separately)."
}
finally {
    if ($null -eq $previousCargoTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    else { $env:CARGO_TARGET_DIR = $previousCargoTarget }
}
