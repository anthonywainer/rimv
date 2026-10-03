[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('CheckPublisherTrust', 'TrustPublisher', 'Install', 'Uninstall')][string] $Action,
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
    if (Get-ChildItem -LiteralPath 'Cert:\LocalMachine\TrustedPeople' -ErrorAction SilentlyContinue |
        Where-Object Thumbprint -eq $thumbprint | Select-Object -First 1) {
        exit 0
    }

    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
    if ($certificate.Subject -ne $certificate.Issuer -and $chain.Build($certificate)) { exit 0 }
    exit 1
}
elseif ($Action -eq 'TrustPublisher') {
    $certificatePath = Join-Path $InstallLocation 'RimV.Identity.cer'
    if (-not (Test-Path -LiteralPath $certificatePath -PathType Leaf)) {
        throw "The RimV identity publisher certificate is missing: $certificatePath"
    }

    $certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certificatePath)
    $trustedCertificate = Get-ChildItem -LiteralPath 'Cert:\LocalMachine\TrustedPeople' -ErrorAction SilentlyContinue |
        Where-Object Thumbprint -eq $certificate.Thumbprint | Select-Object -First 1
    if ($trustedCertificate) { exit 0 }

    try {
        $certutil = Join-Path $env:WINDIR 'System32\certutil.exe'
        $quotedCertificatePath = '"' + $certificatePath + '"'
        $process = Start-Process -FilePath $certutil -Verb RunAs `
            -ArgumentList "-addstore -f TrustedPeople $quotedCertificatePath" `
            -Wait -PassThru
        if ($process.ExitCode -ne 0) { exit $process.ExitCode }
    }
    catch {
        exit 1
    }
    $trustedCertificate = Get-ChildItem -LiteralPath 'Cert:\LocalMachine\TrustedPeople' -ErrorAction SilentlyContinue |
        Where-Object Thumbprint -eq $certificate.Thumbprint | Select-Object -First 1
    if (-not $trustedCertificate) { exit 1 }
    exit 0
}
elseif ($Action -eq 'Install') {
    $packagePath = Join-Path $InstallLocation 'RimV.Identity.msix'
    if (-not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
        throw "The signed RimV identity package is missing: $packagePath"
    }
    try {
        Add-AppxPackage -Path $packagePath -ExternalLocation $InstallLocation -ForceUpdateFromAnyVersion -ErrorAction Stop
    }
    catch {
        # NSIS only receives PowerShell's process exit code. Preserve a small,
        # non-sensitive diagnostic beside the package so installer failures
        # can report the actual AppX boundary and HRESULT.
        try {
            [pscustomobject]@{
                operation = 'Add-AppxPackage'
                errorType = $_.Exception.GetType().FullName
                hresult = '0x' + $_.Exception.HResult.ToString('X8')
            } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $InstallLocation 'RimV.IdentityRegistration.json') -Encoding ascii
        }
        catch { }
        throw
    }
}
else {
    Get-AppxPackage -Name $packageName | Remove-AppxPackage -ErrorAction SilentlyContinue
}
