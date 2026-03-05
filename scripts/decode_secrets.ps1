if (-not $env:PARAMETERS_SECRET) {
    Write-Host "PARAMETERS_SECRET not set, reading from 1password"
    $env:PARAMETERS_SECRET = op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password"
}

gpg --batch --yes --passphrase $env:PARAMETERS_SECRET --decrypt ./.github/secrets/parameters_aws.json.gpg | Out-File -FilePath parameters.json -Encoding ascii
gpg --batch --yes --passphrase $env:PARAMETERS_SECRET --decrypt tests/performance/parameters/parameters_perf_aws.json.gpg | Out-File -FilePath tests/performance/parameters/parameters_perf_aws.json -Encoding ascii
gpg --batch --yes --passphrase $env:PARAMETERS_SECRET --decrypt tests/performance/parameters/parameters_perf_azure.json.gpg | Out-File -FilePath tests/performance/parameters/parameters_perf_azure.json -Encoding ascii
gpg --batch --yes --passphrase $env:PARAMETERS_SECRET --decrypt tests/performance/parameters/parameters_perf_gcp.json.gpg | Out-File -FilePath tests/performance/parameters/parameters_perf_gcp.json -Encoding ascii
