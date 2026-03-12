param(
    [ValidateSet("aws", "gcp", "azure")]
    [string]$Cloud = "aws"
)

if (-not $env:PARAMETERS_SECRET) {
    Write-Error "PARAMETERS_SECRET environment variable is not set"
    exit 1
}

$ErrorActionPreference = "Stop"

Write-Host "Decoding secrets with GPG..."

# Decode main parameters file (required)
$env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "parameters.json" --decrypt "./.github/secrets/parameters_$Cloud.json.gpg"
if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for ./.github/secrets/parameters_$Cloud.json.gpg with exit code $LASTEXITCODE" }
Write-Host "  ✓ parameters.json"

# Decode performance test parameters if they exist (optional)
$perfDir = "tests/performance/parameters"
if (Test-Path "$perfDir/parameters_perf_aws.json.gpg") {
    $env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "$perfDir/parameters_perf_aws.json" --decrypt "$perfDir/parameters_perf_aws.json.gpg"
    if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for $perfDir/parameters_perf_aws.json.gpg with exit code $LASTEXITCODE" }
    Write-Host "  ✓ parameters_perf_aws.json"
} else {
    Write-Host "  ⊘ parameters_perf_aws.json.gpg not found, skipping"
}

if (Test-Path "$perfDir/parameters_perf_azure.json.gpg") {
    $env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "$perfDir/parameters_perf_azure.json" --decrypt "$perfDir/parameters_perf_azure.json.gpg"
    if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for $perfDir/parameters_perf_azure.json.gpg with exit code $LASTEXITCODE" }
    Write-Host "  ✓ parameters_perf_azure.json"
} else {
    Write-Host "  ⊘ parameters_perf_azure.json.gpg not found, skipping"
}

if (Test-Path "$perfDir/parameters_perf_gcp.json.gpg") {
    $env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "$perfDir/parameters_perf_gcp.json" --decrypt "$perfDir/parameters_perf_gcp.json.gpg"
    if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for $perfDir/parameters_perf_gcp.json.gpg with exit code $LASTEXITCODE" }
    Write-Host "  ✓ parameters_perf_gcp.json"
} else {
    Write-Host "  ⊘ parameters_perf_gcp.json.gpg not found, skipping"
}

Write-Host "Successfully decoded all secret files"
