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

# ── Fork path: plaintext per-cloud parameters (PARAMETERS_JSON_<CLOUD>) ──────
# A fork of the public mirror cannot decrypt the committed .github/secrets/*.gpg
# bundle: that needs the PARAMETERS_SECRET passphrase, which is not (and must
# not be) available outside the maintainers' own CI. Instead a fork owner runs
# CI against their OWN account by setting a repository secret
# PARAMETERS_JSON_<CLOUD> (e.g. PARAMETERS_JSON_AWS) whose value is the full
# contents of a parameters.json for that cloud. Provide one per cloud you want
# to exercise (PARAMETERS_JSON_AWS / _GCP / _AZURE) — see CONTRIBUTING.md.
#
# Precedence: plaintext PARAMETERS_JSON_<CLOUD>  >  PARAMETERS_SECRET (GPG)  >
# 1Password. The maintainers' own CI sets PARAMETERS_SECRET and never sets
# PARAMETERS_JSON_*, so this branch is inert there and the GPG path below is
# byte-for-byte unchanged.
#
# When this branch is taken we deliberately skip the GPG decrypt AND the bulk
# directory decode (decode_dir) below — both require the passphrase a fork does
# not have. Tests read the account via PARAMETER_PATH=<...>/parameters.json.
CLOUD_UPPER="$(printf '%s' "${CLOUD}" | tr '[:lower:]' '[:upper:]')"
PLAINTEXT_VAR="PARAMETERS_JSON_${CLOUD_UPPER}"
if [[ -n "${!PLAINTEXT_VAR:-}" ]]; then
    echo "Using plaintext ${PLAINTEXT_VAR} (fork path) — skipping GPG bundle"
    printf '%s' "${!PLAINTEXT_VAR}" > "${OUTPUT_FILE}"
    echo "  ✓ ${OUTPUT_FILE} (from ${PLAINTEXT_VAR})"
    echo "Successfully wrote parameters from plaintext secret"
    exit 0
fi

# Read param secret from 1password if not set
if [ -z "${PARAMETERS_SECRET:-}" ]; then
    echo "PARAMETERS_SECRET not set, reading from 1password"
    PARAMETERS_SECRET=$(op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password")
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
