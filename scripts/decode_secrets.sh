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
    PARAMETERS_SECRET=$(op read "op://<vault>/PARAMETERS_SECRET/password")
fi

echo "Decoding secrets with GPG..."

# Auto-detect CI vs local execution.
# CI: GitHub Actions sets GITHUB_ACTIONS=true; Jenkins sets BUILD_NUMBER.
# Local: use parameters_<cloud>_local.json.gpg (preserves sfctest0 access for dev).
# CI:    use parameters_<cloud>.json.gpg (dedicated prod accounts).
if [[ "${GITHUB_ACTIONS:-}" == "true" || -n "${BUILD_NUMBER:-}" ]]; then
    GPG_SUFFIX=""
    echo "  CI environment detected — using dedicated prod account credentials"
else
    GPG_SUFFIX="_local"
    echo "  Local environment detected — using local/sfctest0 credentials"
fi

GPG_FILE="./.github/secrets/parameters_${CLOUD}${GPG_SUFFIX}.json.gpg"

# Decode main parameters file to repo root (required — callers expect parameters.json)
printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "${GPG_FILE}" > "${OUTPUT_FILE}"
echo "  ✓ ${OUTPUT_FILE} (from ${GPG_FILE})"

# Decode every other GPG file found in the same directory and in tests/performance/parameters/,
# placing the plaintext next to the encrypted file (same directory).
decode_dir() {
    local dir="$1"
    for gpg_file in "${dir}"/*.json.gpg; do
        [ -f "${gpg_file}" ] || continue
        local out="${gpg_file%.gpg}"
        printf '%s' "${PARAMETERS_SECRET}" | gpg --batch --yes --passphrase-fd 0 --decrypt "${gpg_file}" > "${out}"
        echo "  ✓ ${out}"
    done
}

decode_dir "./.github/secrets"

perf_dir="tests/performance/parameters"
if [ -d "${perf_dir}" ]; then
    decode_dir "${perf_dir}"
else
    echo "  ⊘ ${perf_dir} not found, skipping"
fi

echo "Successfully decoded all secret files"
