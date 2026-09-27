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
$cargoManifest = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw
$versionMatch = [regex]::Match($cargoManifest, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)"\s*$')
if (-not $versionMatch.Success) { throw 'Could not read the canonical semantic version from Cargo.toml.' }
$appVersion = $versionMatch.Groups['version'].Value
$numericMatch = [regex]::Match($appVersion, '^(?<major>\d+)\.(?<minor>\d+)\.(?<patch>\d+)(?:-[0-9A-Za-z.-]+)?$')
if (-not $numericMatch.Success) { throw "Unsupported application version: $appVersion" }
$numericVersion = '{0}.{1}.{2}.0' -f $numericMatch.Groups['major'].Value, $numericMatch.Groups['minor'].Value, $numericMatch.Groups['patch'].Value

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
        --locked-mode `
        --runtime win-x64 `
        -p:Platform=x64 `
        -p:SelfContained=true `
        -p:WindowsAppSDKSelfContained=true `
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
        signed = $false
        nativeRuntimeDlls = $nativeDllNames
        files = $payloadFiles
    }

    $manifestPath = Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-manifest.json"
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    $makensis = if ($env:MAKENSIS_EXE) { $env:MAKENSIS_EXE } else { Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
    if (-not (Test-Path -LiteralPath $makensis)) { throw "NSIS 3.11 compiler not found: $makensis" }
    $icon = Join-Path $repoRoot 'apps\windows\icons\icon.ico'
    & $makensis "/DPAYLOADDIR=$payloadDirectory" "/DARTIFACTDIR=$artifactDirectory" "/DAPP_VERSION=$appVersion" "/DNUMERIC_VERSION=$numericVersion" "/DAPP_ICON=$icon" $installerScript
    if ($LASTEXITCODE -ne 0) { throw 'NSIS failed to build the native Windows installer.' }

    $installerPath = Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-setup.exe"
    if (-not (Test-Path -LiteralPath $installerPath)) { throw 'NSIS did not produce the expected installer.' }
    $checksums = @($installerPath, $manifestPath) | ForEach-Object {
        '{0}  {1}' -f (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant(), (Split-Path $_ -Leaf)
    }
    $checksums | Set-Content -LiteralPath (Join-Path $artifactDirectory "RimV-$appVersion-windows-x64-SHA256SUMS.txt") -Encoding ascii
    Write-Host "Created the x64 installer and manifest under $artifactDirectory (unsigned unless Authenticode is configured separately)."
}
finally {
    if ($null -eq $previousCargoTarget) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue }
    else { $env:CARGO_TARGET_DIR = $previousCargoTarget }
}
