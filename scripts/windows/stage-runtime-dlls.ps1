$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$targetRoot = if ($env:CARGO_TARGET_DIR) {
    (Resolve-Path $env:CARGO_TARGET_DIR).Path
} else {
    Join-Path $repoRoot 'target'
}
$releaseDir = Join-Path $targetRoot 'release'
$sourceDlls = @(Get-ChildItem -LiteralPath $releaseDir -Filter '*.dll' -File)
if ($sourceDlls.Count -eq 0) {
    throw "No Windows ASR runtime DLLs were produced in $releaseDir. Refusing to create an incomplete installer."
}

$runtimeDir = Join-Path $repoRoot 'apps\windows\runtime'
New-Item -ItemType Directory -Path $runtimeDir -Force | Out-Null
Get-ChildItem -LiteralPath $runtimeDir -Filter '*.dll' -File | Remove-Item -Force
Remove-Item -LiteralPath (Join-Path $runtimeDir 'runtime-dlls.txt') -Force -ErrorAction SilentlyContinue
foreach ($dll in $sourceDlls) {
    Copy-Item -LiteralPath $dll.FullName -Destination $runtimeDir
}
$sourceDlls.Name | Sort-Object | Set-Content -LiteralPath (Join-Path $runtimeDir 'runtime-dlls.txt') -Encoding ascii
Write-Host "Staged $($sourceDlls.Count) Windows runtime DLL(s) for the installer."
