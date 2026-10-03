[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('CheckPublisherTrust', 'Install', 'Uninstall')][string] $Action,
    [Parameter(Mandatory = $true)][string] $InstallLocation
)

$ErrorActionPreference = 'Stop'
$packageName = 'RimV.Native.Windows'

if ($Action -eq 'CheckPublisherTrust') {
    $certificatePath = Join-Path $InstallLocation 'RimV.Identity.cer'
    if (-not (Test-Path -LiteralPath $certificatePath -PathType Leaf)) {
        throw "The RimV identity publisher certificate is missing: $certificatePath"
    }

    $certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certificatePath)
    $thumbprint = $certificate.Thumbprint
    foreach ($storePath in @('Cert:\CurrentUser\TrustedPeople', 'Cert:\LocalMachine\TrustedPeople')) {
        if (Get-ChildItem -LiteralPath $storePath -ErrorAction SilentlyContinue |
            Where-Object Thumbprint -eq $thumbprint | Select-Object -First 1) {
            exit 0
        }
    }

    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    if ($chain.Build($certificate)) { exit 0 }
    exit 1
}
elseif ($Action -eq 'Install') {
    $packagePath = Join-Path $InstallLocation 'RimV.Identity.msix'
    if (-not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
        throw "The signed RimV identity package is missing: $packagePath"
    }
    Add-AppxPackage -Path $packagePath -ExternalLocation $InstallLocation -ForceUpdateFromAnyVersion
}
else {
    Get-AppxPackage -Name $packageName | Remove-AppxPackage -ErrorAction SilentlyContinue
}
