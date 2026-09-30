#!/bin/bash -e
#
# Runs inside Docker with Node installed on a WIF cloud VM.
# Expects /tests/nodejs_wif.tar.gz and /tests/parameters.json.

set -euo pipefail

TESTS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORK_DIR=/tmp/nodejs_wif

mkdir -p "$WORK_DIR"
tar -xzf "$TESTS_DIR/nodejs_wif.tar.gz" -C "$WORK_DIR"
cd "$WORK_DIR/nodejs"

export PARAMETER_PATH="$TESTS_DIR/parameters.json"
export SKIP_NODEJS_BUILD=true

npm run test:e2e -- tests/e2e/authentication/workload-identity.test.ts
