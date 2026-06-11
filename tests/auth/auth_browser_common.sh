#!/bin/bash
#
# Common setup for authentication E2E tests requiring a headless browser.
# Sourced by wrapper-specific scripts (auth_browser_python.sh, etc.)
#
# Expects:
#   - parameters_preprod.json in workspace root (from decode_secrets.sh)
#   - Running inside the snowdrivers-test-external-browser-universal-driver image

set -euo pipefail

WORKSPACE_ROOT="${WORKSPACE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

if [ ! -f "${WORKSPACE_ROOT}/parameters_preprod.json" ]; then
    echo "ERROR: parameters_preprod.json not found in ${WORKSPACE_ROOT}" >&2
    echo "Run: ./scripts/decode_secrets.sh preprod parameters_preprod.json" >&2
    exit 1
fi

export PARAMETER_PATH="${WORKSPACE_ROOT}/parameters_preprod.json"
export SF_TEST_HEADLESS_BROWSER=true
export CARGO_TARGET_DIR="${WORKSPACE_ROOT}/target"
