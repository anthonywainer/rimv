[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('Install', 'Uninstall')][string] $Action,
    [Parameter(Mandatory = $true)][string] $InstallLocation
)

$ErrorActionPreference = 'Stop'
$packageName = 'RimV.Native.Windows'

if ($Action -eq 'Install') {
    $packagePath = Join-Path $InstallLocation 'RimV.Identity.msix'
    if (-not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
        throw "The signed RimV identity package is missing: $packagePath"
    }
    Add-AppxPackage -Path $packagePath -ExternalLocation $InstallLocation -ForceUpdateFromAnyVersion
}
else {
    Get-AppxPackage -Name $packageName | Remove-AppxPackage -ErrorAction SilentlyContinue
}
