#!/bin/bash
CLOUD="${1:-aws}"

if [[ "${CLOUD}" != "aws" && "${CLOUD}" != "gcp" && "${CLOUD}" != "azure" ]]; then
    echo "Usage: $0 [aws|gcp|azure]" >&2
    exit 1
fi

set -euo pipefail

# Read param secret from 1password if not set
if [ -z "${PARAMETERS_SECRET:-}" ]; then
    echo "PARAMETERS_SECRET not set, reading from 1password"
    PARAMETERS_SECRET=$(op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password")
fi

echo "Decoding secrets with GPG..."

# Decode main parameters file (required)
printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "./.github/secrets/parameters_${CLOUD}.json.gpg" > parameters.json
echo "  ✓ parameters.json"

# Decode performance test parameters if they exist (optional)
perf_dir="tests/performance/parameters"
if [ -f "$perf_dir/parameters_perf_aws.json.gpg" ]; then
    printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_aws.json.gpg" > "$perf_dir/parameters_perf_aws.json"
    echo "  ✓ parameters_perf_aws.json"
else
    echo "  ⊘ parameters_perf_aws.json.gpg not found, skipping"
fi

if [ -f "$perf_dir/parameters_perf_azure.json.gpg" ]; then
    printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_azure.json.gpg" > "$perf_dir/parameters_perf_azure.json"
    echo "  ✓ parameters_perf_azure.json"
else
    echo "  ⊘ parameters_perf_azure.json.gpg not found, skipping"
fi

if [ -f "$perf_dir/parameters_perf_gcp.json.gpg" ]; then
    printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_gcp.json.gpg" > "$perf_dir/parameters_perf_gcp.json"
    echo "  ✓ parameters_perf_gcp.json"
else
    echo "  ⊘ parameters_perf_gcp.json.gpg not found, skipping"
fi

echo "Successfully decoded all secret files"
