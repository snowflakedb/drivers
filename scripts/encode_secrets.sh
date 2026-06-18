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
    PARAMETERS_SECRET=$(op read "op://<vault>/PARAMETERS_SECRET/password")
fi

echo "Encoding secrets with GPG..."

gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --symmetric --cipher-algo AES256 -o "./.github/secrets/parameters_${CLOUD}.json.gpg" parameters.json
echo "  ✓ .github/secrets/parameters_${CLOUD}.json.gpg"

echo "Successfully encoded secret file"
