$ErrorActionPreference = 'Stop'

cargo build --release --locked -p engine-runtime
if ($LASTEXITCODE -ne 0) {
    throw 'Failed to build the Windows transcription runtime before bundling.'
}

& (Join-Path $PSScriptRoot 'stage-runtime-dlls.ps1')
