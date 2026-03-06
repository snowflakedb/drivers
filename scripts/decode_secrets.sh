#!/bin/bash

CLOUD="${1:-aws}"

if [[ "${CLOUD}" != "aws" && "${CLOUD}" != "gcp" ]]; then
    echo "Usage: $0 [aws|gcp]" >&2
    exit 1
fi

# Read param secret from 1password if not set
if [ -z "${PARAMETERS_SECRET}" ]; then
    echo "PARAMETERS_SECRET not set, reading from 1password"
    PARAMETERS_SECRET=$(op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password")
fi

gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt "./.github/secrets/parameters_${CLOUD}.json.gpg" > parameters.json
gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt tests/performance/parameters/parameters_perf_aws.json.gpg > tests/performance/parameters/parameters_perf_aws.json
gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt tests/performance/parameters/parameters_perf_azure.json.gpg > tests/performance/parameters/parameters_perf_azure.json
gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt tests/performance/parameters/parameters_perf_gcp.json.gpg > tests/performance/parameters/parameters_perf_gcp.json
