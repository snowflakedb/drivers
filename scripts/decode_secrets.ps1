#!/usr/bin/env pwsh

# Decode secrets using GPG on Windows
# This script is the Windows equivalent of decode_secrets.sh

$ErrorActionPreference = "Stop"

if (-not $env:PARAMETERS_SECRET) {
    Write-Error "PARAMETERS_SECRET environment variable is not set"
    exit 1
}

Write-Host "Decoding secrets with GPG..."

# Decode main parameters file
gpg --batch --yes --passphrase $env:PARAMETERS_SECRET --decrypt --output parameters.json .github/secrets/parameters_aws.json.gpg

Write-Host "Successfully decoded parameters.json"
