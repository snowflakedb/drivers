#!/bin/bash

set -euo pipefail

# Read param secret from 1password if not set
if [ -z "${PARAMETERS_SECRET}" ]; then
    echo "PARAMETERS_SECRET not set, reading from 1password"
    PARAMETERS_SECRET=$(op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password")
fi

echo "Decoding secrets with GPG..."

# Decode main parameters file (required)
echo "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt ./.github/secrets/parameters_aws.json.gpg > parameters.json
echo "  ✓ parameters.json"

# Decode performance test parameters if they exist (optional)
perf_dir="tests/performance/parameters"
if [ -f "$perf_dir/parameters_perf_aws.json.gpg" ]; then
    echo "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_aws.json.gpg" > "$perf_dir/parameters_perf_aws.json"
    echo "  ✓ parameters_perf_aws.json"
else
    echo "  ⊘ parameters_perf_aws.json.gpg not found, skipping"
fi

if [ -f "$perf_dir/parameters_perf_azure.json.gpg" ]; then
    echo "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_azure.json.gpg" > "$perf_dir/parameters_perf_azure.json"
    echo "  ✓ parameters_perf_azure.json"
else
    echo "  ⊘ parameters_perf_azure.json.gpg not found, skipping"
fi

if [ -f "$perf_dir/parameters_perf_gcp.json.gpg" ]; then
    echo "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "$perf_dir/parameters_perf_gcp.json.gpg" > "$perf_dir/parameters_perf_gcp.json"
    echo "  ✓ parameters_perf_gcp.json"
else
    echo "  ⊘ parameters_perf_gcp.json.gpg not found, skipping"
fi

echo "Successfully decoded all secret files"
