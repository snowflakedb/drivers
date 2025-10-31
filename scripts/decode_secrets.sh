#!/bin/bash

# Read param secret from 1password if not set
if [ -z "${PARAMETERS_SECRET}" ]; then
    echo "PARAMETERS_SECRET not set, reading from 1password"
    PARAMETERS_SECRET=$(op read "op://Eng - Snow Drivers Warsaw/PARAMETERS_SECRET/password")
fi

gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt ./.github/secrets/parameters_aws.json.gpg > parameters.json
gpg --batch --yes --passphrase "${PARAMETERS_SECRET}" --decrypt tests/performance/parameters/parameters_perf_aws.json.gpg > tests/performance/parameters/parameters_perf_aws.json
