#!/usr/bin/env pwsh

# Decode secrets using GPG on Windows
# Decrypts test parameter files needed for CI testing

$ErrorActionPreference = "Stop"

if (-not $env:PARAMETERS_SECRET) {
    Write-Error "PARAMETERS_SECRET environment variable is not set"
    exit 1
}

Write-Host "Decoding secrets with GPG..."

# Helper function to decrypt with proper error handling
function Invoke-GpgDecrypt {
    param(
        [string]$InputFile,
        [string]$OutputFile,
        [string]$Passphrase
    )

    # Use Write-Output to properly pipe passphrase to gpg stdin
    # This mimics bash's "echo '$PASS' | gpg --passphrase-fd 0"
    $result = Write-Output $Passphrase | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --decrypt --output $OutputFile $InputFile 2>&1

    if ($LASTEXITCODE -ne 0) {
        Write-Error "GPG decryption failed for $InputFile : $result"
        exit 1
    }
}

# Decode main parameters file
Invoke-GpgDecrypt -InputFile ".github/secrets/parameters_aws.json.gpg" -OutputFile "parameters.json" -Passphrase $env:PARAMETERS_SECRET
Write-Host "  ✓ parameters.json"

# Decode performance test parameters if they exist
$perfDir = "tests/performance/parameters"
if (Test-Path "$perfDir/parameters_perf_aws.json.gpg") {
    Invoke-GpgDecrypt -InputFile "$perfDir/parameters_perf_aws.json.gpg" -OutputFile "$perfDir/parameters_perf_aws.json" -Passphrase $env:PARAMETERS_SECRET
    Write-Host "  ✓ parameters_perf_aws.json"
}
if (Test-Path "$perfDir/parameters_perf_azure.json.gpg") {
    Invoke-GpgDecrypt -InputFile "$perfDir/parameters_perf_azure.json.gpg" -OutputFile "$perfDir/parameters_perf_azure.json" -Passphrase $env:PARAMETERS_SECRET
    Write-Host "  ✓ parameters_perf_azure.json"
}
if (Test-Path "$perfDir/parameters_perf_gcp.json.gpg") {
    Invoke-GpgDecrypt -InputFile "$perfDir/parameters_perf_gcp.json.gpg" -OutputFile "$perfDir/parameters_perf_gcp.json" -Passphrase $env:PARAMETERS_SECRET
    Write-Host "  ✓ parameters_perf_gcp.json"
}

Write-Host "Successfully decoded all secret files"
