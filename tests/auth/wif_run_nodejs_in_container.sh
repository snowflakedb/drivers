#!/bin/bash -e
#
# Runs inside Docker with Node installed on a WIF cloud VM.
# Expects /tests/nodejs_wif.tar.gz, /tests/parameters.json, and
# /tests/nodejs_test_args (NUL-separated extra Vitest arguments from
# run_wif.sh, empty when none are given).
# VITEST_PROJECT selects e2e (universal) or e2e-old-driver (reference).

set -euo pipefail

TESTS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORK_DIR=/tmp/nodejs_wif

log() {
  printf '[wif-vm][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

log "Node $(node --version) on $(uname -m)"

log "Unpacking nodejs_wif.tar.gz ($(du -h "$TESTS_DIR/nodejs_wif.tar.gz" | cut -f1))"
unpack_started=$SECONDS
mkdir -p "$WORK_DIR"
tar -xzf "$TESTS_DIR/nodejs_wif.tar.gz" -C "$WORK_DIR"
cd "$WORK_DIR/nodejs"
log "Unpacked in $((SECONDS - unpack_started))s"

export PARAMETER_PATH="$TESTS_DIR/parameters.json"
export SKIP_NODEJS_BUILD=true

NPM_SCRIPT=test:e2e
if [[ "${VITEST_PROJECT:-e2e}" == e2e-old-driver ]]; then
  NPM_SCRIPT=test:e2e-old-driver
fi

if [[ ! -f "$TESTS_DIR/nodejs_test_args" ]]; then
  echo "ERROR: missing $TESTS_DIR/nodejs_test_args (Vitest arguments from run_wif.sh)" >&2
  exit 1
fi
mapfile -d '' -t vitest_args < "$TESTS_DIR/nodejs_test_args"

log "Starting npm run ${NPM_SCRIPT} ${vitest_args[*]}"
npm run "${NPM_SCRIPT}" -- tests/e2e/authentication/workload-identity.test.ts "${vitest_args[@]}"
