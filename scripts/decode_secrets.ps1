param(
    [ValidateSet("aws", "gcp", "azure")]
    [string]$Cloud = "aws"
)

if (-not $env:PARAMETERS_SECRET) {
    Write-Host "PARAMETERS_SECRET not set, reading from 1password"
    $env:PARAMETERS_SECRET = op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password"
}

$env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "parameters.json" --decrypt "./.github/secrets/parameters_$Cloud.json.gpg"
if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for ./.github/secrets/parameters_$Cloud.json.gpg with exit code $LASTEXITCODE" }
$env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "tests/performance/parameters/parameters_perf_aws.json" --decrypt "tests/performance/parameters/parameters_perf_aws.json.gpg"
if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for tests/performance/parameters/parameters_perf_aws.json.gpg with exit code $LASTEXITCODE" }
$env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "tests/performance/parameters/parameters_perf_azure.json" --decrypt "tests/performance/parameters/parameters_perf_azure.json.gpg"
if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for tests/performance/parameters/parameters_perf_azure.json.gpg with exit code $LASTEXITCODE" }
$env:PARAMETERS_SECRET | gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 --output "tests/performance/parameters/parameters_perf_gcp.json" --decrypt "tests/performance/parameters/parameters_perf_gcp.json.gpg"
if ($LASTEXITCODE -ne 0) { throw "gpg decryption failed for tests/performance/parameters/parameters_perf_gcp.json.gpg with exit code $LASTEXITCODE" }
