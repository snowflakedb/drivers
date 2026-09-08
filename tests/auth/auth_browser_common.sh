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

# Diagnostics only: log setup so Jenkins can be diagnosed from the console.
# Does not fail the job when overlay keys are missing (print-only).
echo "=== Auth-browser diagnostics: environment ==="
echo "  WORKSPACE_ROOT:          ${WORKSPACE_ROOT}"
echo "  PARAMETER_PATH:          ${PARAMETER_PATH}"
echo "  AUTH_BROWSER_MODE:       ${AUTH_BROWSER_MODE:-universal}"
echo "  SF_TEST_HEADLESS_BROWSER:${SF_TEST_HEADLESS_BROWSER}"
echo "  BUILD_TAG:               ${BUILD_TAG:-unset}"
echo "  node:                    $(node --version 2>/dev/null || echo 'not installed')"
echo "  python3:                 $(python3 --version 2>/dev/null || echo 'not installed')"

# A wrapper overlays testconnection-<language> on testconnection. Print which
# section defines each key (present/empty/absent) — never values.
#
# TODO(SNOW-3996212): skip-all while SF_TEST_HEADLESS_BROWSER is unset is a
# green vitest; fail that in auth_browser_nodejs.sh after the run if the whole
# run executed zero tests. This inventory stays print-only and must not fail
# on missing overlay keys.
_AUTH_COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "${_AUTH_COMMON_DIR}/auth_browser_param_diagnostics.py"
