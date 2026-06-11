#!/bin/bash
CLOUD="${1:-aws}"
# Output file for the main parameters bundle. Defaults to parameters.json so every
# existing caller (GitHub workflows, ODBC/JDBC/Rust runners, etc.) is unaffected.
# Pass a distinct name (e.g. parameters_preprod.json) to decode an alternate account
# WITHOUT clobbering a parameters.json already decoded for the rest of the suite.
OUTPUT_FILE="${2:-parameters.json}"

if [[ "${CLOUD}" != "aws" && "${CLOUD}" != "gcp" && "${CLOUD}" != "azure" && "${CLOUD}" != "preprod" ]]; then
    echo "Usage: $0 [aws|gcp|azure|preprod] [output-file]" >&2
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
printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "./.github/secrets/parameters_${CLOUD}.json.gpg" > "${OUTPUT_FILE}"
echo "  ✓ ${OUTPUT_FILE}"

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
