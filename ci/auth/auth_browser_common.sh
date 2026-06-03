#!/bin/bash
#
# Common setup for authentication E2E tests requiring a headless browser.
# Sourced by wrapper-specific scripts (auth_browser_python.sh, etc.)
#
# Expects:
#   - parameters.json in workspace root (from decode_secrets.sh)
#   - Running inside the snowdrivers-test-external-browser-universal-driver image

set -euo pipefail

WORKSPACE_ROOT="${WORKSPACE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

if [ ! -f "${WORKSPACE_ROOT}/parameters.json" ]; then
    echo "ERROR: parameters.json not found in ${WORKSPACE_ROOT}" >&2
    echo "Run scripts/decode_secrets.sh first." >&2
    exit 1
fi

export PARAMETER_PATH="${WORKSPACE_ROOT}/parameters.json"
export SF_TEST_HEADLESS_BROWSER=true
export CARGO_TARGET_DIR="${WORKSPACE_ROOT}/target"
