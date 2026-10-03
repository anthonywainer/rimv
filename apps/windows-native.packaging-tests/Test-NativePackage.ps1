[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Payload', 'Installer')]
    [string] $Mode,

    [Parameter(Mandatory = $true)]
    [string] $ManifestPath,

    [string] $PayloadDirectory,
    [string] $InstallerPath,
    [string] $InstallDirectory = (Join-Path $env:TEMP 'RimV Native Windows Install With Spaces')
)

$ErrorActionPreference = 'Stop'

function Assert-Condition([bool] $condition, [string] $message) {
    if (-not $condition) { throw $message }
}

function Get-PeInfo([string] $path) {
    $stream = [IO.File]::OpenRead($path)
    try {
        $reader = [IO.BinaryReader]::new($stream)
        $stream.Position = 0x3c
        $peOffset = $reader.ReadInt32()
        $stream.Position = $peOffset
        Assert-Condition ($reader.ReadUInt32() -eq 0x00004550) "Invalid PE signature: $path"
        $machine = $reader.ReadUInt16()
        $stream.Position = $peOffset + 24 + 68
        $subsystem = $reader.ReadUInt16()
        return [pscustomobject]@{ Machine = $machine; Subsystem = $subsystem }
    }
    finally { $stream.Dispose() }
}

function Assert-Payload([string] $root, $manifest) {
    Assert-Condition (Test-Path -LiteralPath $root -PathType Container) "Payload directory does not exist: $root"
    Assert-Condition ($manifest.schemaVersion -eq 1) 'Unsupported package manifest schema.'
    Assert-Condition ($manifest.architecture -eq 'x64') 'This installer is expected to target x64.'
    Assert-Condition ($manifest.ffiApiVersion -eq 1) 'The package does not declare the expected shared-core API version.'
    $optionalModelIds = @($manifest.optionalModels | ForEach-Object id | Sort-Object)
    Assert-Condition (($optionalModelIds -join ',') -eq 'parakeet-tdt-0.6b-v3-int8,whisper-base') 'The installer optional-model choices do not match the shared catalog.'
    Assert-Condition (@($manifest.optionalModels | Where-Object { $_.sizeBytes -le 0 }).Count -eq 0) 'An optional-model download size is missing from the shared catalog.'

    $expected = @{}
    foreach ($file in $manifest.files) {
        $relative = $file.path.Replace('/', [IO.Path]::DirectorySeparatorChar)
        $fullPath = Join-Path $root $relative
        Assert-Condition (Test-Path -LiteralPath $fullPath -PathType Leaf) "Package payload is missing $relative"
        $actual = (Get-FileHash -LiteralPath $fullPath -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert-Condition ($actual -eq $file.sha256) "Payload hash mismatch for $relative"
        $expected[$file.path] = $true
    }

    $actualFiles = @(Get-ChildItem -LiteralPath $root -File -Recurse | ForEach-Object {
        [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/')
    })
    Assert-Condition ($actualFiles.Count -eq $expected.Count) 'Payload file count differs from the manifest.'
    foreach ($path in $actualFiles) {
        Assert-Condition $expected.ContainsKey($path) "Unexpected file is present in the production payload: $path"
        Assert-Condition ($path -notmatch '(?i)(^|/)(tests?|fixtures?|mock)s?(/|$)|\.(pdb|ilk|exp|iobj|ipdb)$|testhost|coverlet') "Test-only/debug file found in production payload: $path"
        $knownWinUiRuntimeAsset = $path -ieq 'Microsoft.UI.Xaml/Assets/map.html'
        $legacyFrontendAsset = $path -match '(?i)(^|/)(tauri|node_modules|vite)(/|$)|\.(html|js|css)$'
        Assert-Condition (-not $legacyFrontendAsset -or $knownWinUiRuntimeAsset) "Legacy HTML/Tauri asset found in native payload: $path"
        Assert-Condition ($path -notmatch '(?i)Assets\.xcassets|\.(onnx|ggml|tar\.bz2)$') "macOS assets or optional model data found in base payload: $path"
    }

    $appExe = Join-Path $root 'RimV.Windows.exe'
    Assert-Condition (Test-Path -LiteralPath $appExe -PathType Leaf) 'The native WinUI executable is missing.'
    Assert-Condition (Test-Path -LiteralPath (Join-Path $root 'RimV.Identity.msix') -PathType Leaf) 'The signed sparse identity package is missing.'
    $appPe = Get-PeInfo $appExe
    Assert-Condition ($appPe.Machine -eq 0x8664) 'The app executable is not x64.'
    Assert-Condition ($appPe.Subsystem -eq 2) 'The app executable is not a GUI/WinExe program.'

    $nativeDlls = @($manifest.nativeRuntimeDlls)
    Assert-Condition ($nativeDlls -contains 'rimv_core_ffi.dll') 'The Rust shared-core DLL is absent from the native runtime list.'
    foreach ($dll in $nativeDlls) {
        $dllPath = Join-Path $root $dll
        Assert-Condition (Test-Path -LiteralPath $dllPath -PathType Leaf) "Native runtime DLL is missing: $dll"
        Assert-Condition ((Get-PeInfo $dllPath).Machine -eq 0x8664) "Native runtime DLL is not x64: $dll"
    }

    $dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    Assert-Condition ($null -ne $dumpbin) 'dumpbin.exe is required for the native DLL dependency scan.'
    $packageNames = @($nativeDlls | ForEach-Object { (Split-Path $_ -Leaf).ToLowerInvariant() })
    $nativeImages = @($appExe) + @($nativeDlls | ForEach-Object { Join-Path $root $_ })
    foreach ($image in $nativeImages) {
        $dependencyOutput = & $dumpbin.Source /nologo /dependents $image 2>&1
        Assert-Condition ($LASTEXITCODE -eq 0) "dumpbin dependency scan failed for $(Split-Path $image -Leaf)."
        $dependencies = @($dependencyOutput | ForEach-Object {
            if ([string]$_ -match '^\s+([A-Za-z0-9_.+-]+\.dll)\s*$') { $Matches[1].ToLowerInvariant() }
        } | Sort-Object -Unique)
        foreach ($dependency in $dependencies) {
            $isApiSet = $dependency.StartsWith('api-ms-win-') -or $dependency.StartsWith('ext-ms-win-')
            $isSystemDll = Test-Path -LiteralPath (Join-Path $env:WINDIR "System32\$dependency")
            $isPackaged = $packageNames -contains $dependency
            Assert-Condition ($isApiSet -or $isSystemDll -or $isPackaged) "Unresolved native dependency $dependency imported by $(Split-Path $image -Leaf)."
        }
    }

    Assert-Condition (Test-Path -LiteralPath (Join-Path $root 'Assets\rimv.ico')) 'RimV application icon is missing.'
    Assert-Condition (Test-Path -LiteralPath (Join-Path $root 'licenses\RimV-LICENSE.txt')) 'RimV license is missing.'
    Write-Host "PASS: payload files, hashes, x64 GUI subsystem and native DLL dependencies ($($manifest.files.Count) files)."
}

function Invoke-Installer([string] $path, [string] $arguments, [string] $label) {
    $process = Start-Process -FilePath $path -ArgumentList $arguments -PassThru
    if (-not $process.WaitForExit(180000)) {
        $process.Kill($true)
        $process.WaitForExit(10000) | Out-Null
        throw "$label did not finish within three minutes."
    }
    Assert-Condition ($process.ExitCode -eq 0) "$label exited with code $($process.ExitCode)."
}

function Assert-AppStopped([string] $executablePath) {
    $runningApps = @(Get-CimInstance Win32_Process -Filter "Name = 'RimV.Windows.exe'" | Where-Object {
        $_.ExecutablePath -and [string]::Equals(
            [IO.Path]::GetFullPath($_.ExecutablePath),
            [IO.Path]::GetFullPath($executablePath),
            [StringComparison]::OrdinalIgnoreCase)
    })
    Assert-Condition ($runningApps.Count -eq 0) "RimV is still running from the install directory (PID $($runningApps.ProcessId -join ', '))."

    $mutex = $null
    try {
        $mutex = [Threading.Mutex]::OpenExisting('Local\RimV.Native.Windows.v01')
        throw 'RimV still owns its single-instance mutex after the process exit.'
    }
    catch [Threading.WaitHandleCannotBeOpenedException] {
        # No process holds the app mutex, so it no longer exists.
    }
    finally {
        if ($mutex) { $mutex.Dispose() }
    }
}

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($Mode -eq 'Payload') {
    if (-not $PayloadDirectory) { throw 'PayloadDirectory is required in Payload mode.' }
    Assert-Payload $PayloadDirectory $manifest
    exit 0
}

if (-not $IsWindows) { throw 'Installer lifecycle tests must run on Windows.' }
if (-not $InstallerPath) { throw 'InstallerPath is required in Installer mode.' }
Assert-Condition (Test-Path -LiteralPath $InstallerPath -PathType Leaf) "Installer does not exist: $InstallerPath"
$expectedInstaller = "RimV-$($manifest.version)-windows-x64-setup.exe"
Assert-Condition ((Split-Path $InstallerPath -Leaf) -eq $expectedInstaller) "Expected installer name $expectedInstaller."
$checksumFile = Join-Path (Split-Path $ManifestPath -Parent) "RimV-$($manifest.version)-windows-x64-SHA256SUMS.txt"
Assert-Condition (Test-Path -LiteralPath $checksumFile -PathType Leaf) 'The distribution SHA256SUMS file is missing.'
foreach ($path in @($InstallerPath, $ManifestPath)) {
    $name = Split-Path $path -Leaf
    $expectedHash = (Get-Content -LiteralPath $checksumFile | Where-Object { $_ -match "\s+$([regex]::Escape($name))$" } | Select-Object -First 1) -replace '\s+.*$', ''
    Assert-Condition ($expectedHash -match '^[0-9a-fA-F]{64}$') "No SHA-256 entry exists for $name."
    $actualHash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    Assert-Condition ($actualHash -eq $expectedHash) "Distribution checksum mismatch for $name."
}

$tempInstallerDirectory = Join-Path $env:TEMP 'RimV Installer Smoke Test With Spaces'
New-Item -ItemType Directory -Path $tempInstallerDirectory -Force | Out-Null
$testInstaller = Join-Path $tempInstallerDirectory $expectedInstaller
Copy-Item -LiteralPath $InstallerPath -Destination $testInstaller -Force
$appExe = Join-Path $InstallDirectory 'RimV.Windows.exe'
$dataDirectory = Join-Path $env:LOCALAPPDATA 'RimV'
$modelsDirectory = Join-Path $dataDirectory 'models'
$modelFilesBefore = if (Test-Path -LiteralPath $modelsDirectory) {
    @(Get-ChildItem -LiteralPath $modelsDirectory -File -Recurse | ForEach-Object {
        [pscustomobject]@{ Path = [IO.Path]::GetRelativePath($modelsDirectory, $_.FullName); Size = $_.Length; Hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash }
    } | Sort-Object Path)
} else { @() }
$sentinel = Join-Path (Join-Path $dataDirectory 'recordings') 'phase4-uninstall-preservation.txt'
$process = $null

try {
    if (Test-Path -LiteralPath $InstallDirectory) { Remove-Item -LiteralPath $InstallDirectory -Recurse -Force }
    $installArgs = "/S /D=$InstallDirectory"
    Invoke-Installer $testInstaller $installArgs 'Clean installer run'
    Assert-Condition (Test-Path -LiteralPath $appExe -PathType Leaf) 'The installer did not place the native app in the requested path containing spaces.'
    Assert-Condition (@(Get-AppxPackage -Name 'RimV.Native.Windows').Count -gt 0) 'The installer did not register RimV package identity.'
    Assert-Condition (Test-Path -LiteralPath (Join-Path $InstallDirectory 'rimv_core_ffi.dll')) 'The installer omitted the shared Rust library.'
    Assert-Condition (Test-Path -LiteralPath (Join-Path $InstallDirectory 'Assets\rimv.ico')) 'The installer omitted the RimV icon.'
    foreach ($file in $manifest.files) {
        $relative = $file.path.Replace('/', [IO.Path]::DirectorySeparatorChar)
        $installedFile = Join-Path $InstallDirectory $relative
        Assert-Condition (Test-Path -LiteralPath $installedFile -PathType Leaf) "Installed app is missing $relative"
        $actualHash = (Get-FileHash -LiteralPath $installedFile -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert-Condition ($actualHash -eq $file.sha256) "Installed file differs from tested payload: $relative"
    }

    $startMenuLink = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)) 'RimV\RimV Native Windows.lnk'
    Assert-Condition (Test-Path -LiteralPath $startMenuLink -PathType Leaf) 'The Start menu shortcut was not installed.'

    $process = Start-Process -FilePath $appExe -WorkingDirectory $InstallDirectory -PassThru
    Start-Sleep -Seconds 8
    $process.Refresh()
    Assert-Condition (-not $process.HasExited) "The installed WinUI process exited on first launch (code $($process.ExitCode))."
    Write-Host 'PASS: installed WinUI process started and remained running for the startup smoke window.'
    $installerBytes = (Get-Item -LiteralPath $InstallerPath).Length
    $installedFiles = @(Get-ChildItem -LiteralPath $InstallDirectory -File -Recurse)
    $installedBytes = [long](($installedFiles | Measure-Object -Property Length -Sum).Sum)
    $mainExecutableBytes = (Get-Item -LiteralPath $appExe).Length
    Write-Host ('SIZE AUDIT: installer={0:N2} MB; installed={1:N2} MB; main executable={2:N2} MB; installed files={3}' -f ($installerBytes / 1e6), ($installedBytes / 1e6), ($mainExecutableBytes / 1e6), $installedFiles.Count)
    $modelFilesAfter = if (Test-Path -LiteralPath $modelsDirectory) {
        @(Get-ChildItem -LiteralPath $modelsDirectory -File -Recurse | ForEach-Object {
            [pscustomobject]@{ Path = [IO.Path]::GetRelativePath($modelsDirectory, $_.FullName); Size = $_.Length; Hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash }
        } | Sort-Object Path)
    } else { @() }
    Assert-Condition ((ConvertTo-Json -InputObject $modelFilesBefore -Compress) -eq (ConvertTo-Json -InputObject $modelFilesAfter -Compress)) 'Silent default installation changed the local model directory; optional downloads must be unchecked by default.'
    Write-Host 'PASS: silent default installation did not download or alter transcription models.'

    # Hosted Windows runners have no reliable interactive tray session to exercise Quit.
    # Stop the smoke process so the installer/uninstaller lifecycle can continue.
    $process.Kill($true)
    if (-not $process.WaitForExit(30000)) {
        throw 'The installed WinUI process did not exit after termination; refusing to test uninstall against a running app.'
    }
    $process.Refresh()
    Assert-Condition $process.HasExited 'The installed WinUI process is still running; refusing to test uninstall.'
    $process.Dispose()
    $process = $null
    Assert-AppStopped $appExe

    $recordingsDirectory = Join-Path $dataDirectory 'recordings'
    New-Item -ItemType Directory -Path $recordingsDirectory -Force | Out-Null
    Set-Content -LiteralPath $sentinel -Value 'This user recording data must survive uninstall.' -Encoding utf8

    Invoke-Installer $testInstaller "/S /D=$InstallDirectory" 'Same-version reinstall'
    Assert-Condition (Test-Path -LiteralPath $sentinel -PathType Leaf) 'Reinstall removed user recordings data.'
    Assert-Condition (Test-Path -LiteralPath $appExe -PathType Leaf) 'Reinstall removed the application executable.'

    $uninstaller = Join-Path $InstallDirectory 'Uninstall.exe'
    Assert-Condition (Test-Path -LiteralPath $uninstaller -PathType Leaf) 'The uninstaller was not registered in the application directory.'
    $registeredInstallDirectory = (Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RimV.Native.Windows' -Name InstallLocation).InstallLocation
    Assert-Condition ([string]::Equals([IO.Path]::GetFullPath($registeredInstallDirectory), [IO.Path]::GetFullPath($InstallDirectory), [StringComparison]::OrdinalIgnoreCase)) "Uninstall registry path '$registeredInstallDirectory' does not match '$InstallDirectory'."
    Assert-AppStopped $appExe
    Invoke-Installer $uninstaller '/S' 'Uninstall'
    Assert-Condition (-not (Test-Path -LiteralPath $appExe)) 'Uninstall left the application executable installed.'
    Assert-Condition (@(Get-AppxPackage -Name 'RimV.Native.Windows').Count -eq 0) 'Uninstall left RimV package identity registered.'
    Assert-Condition (-not (Test-Path -LiteralPath $startMenuLink)) 'Uninstall left the Start menu shortcut behind.'
    Assert-Condition (Test-Path -LiteralPath $sentinel -PathType Leaf) 'Uninstall deleted user recording/model data.'
    Write-Host 'PASS: install path with spaces, reinstall, Start menu integration, uninstall and user-data preservation.'
}
finally {
    if ($process -and -not $process.HasExited) { $process.Kill($true); $process.WaitForExit(10000) | Out-Null }
    if (Test-Path -LiteralPath $InstallDirectory) {
        $cleanupUninstaller = Join-Path $InstallDirectory 'Uninstall.exe'
        if (Test-Path -LiteralPath $cleanupUninstaller) {
            Start-Process -FilePath $cleanupUninstaller -ArgumentList '/S' -Wait -ErrorAction SilentlyContinue
        }
    }
}
