#!/bin/bash
#
# Python authentication E2E tests requiring a headless browser container.
# Runs the external-browser (headless Chromium) suite inside
# snowdrivers-test-external-browser-universal-driver.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

echo "=== Creating virtualenv ==="
# Use an isolated venv rather than --break-system-packages: the base image's
# Python is externally managed (PEP 668) and we must not mutate it.
VENV_DIR="${WORKSPACE_ROOT}/.venv-auth-browser"
python3 -m venv "${VENV_DIR}"
# shellcheck disable=SC1091
source "${VENV_DIR}/bin/activate"

cd "${WORKSPACE_ROOT}/python"

# AUTH_BROWSER_MODE selects which connector to test (default: universal/new):
#   universal -> editable install of this repo's connector (builds sf_core)
#   reference -> legacy snowflake-connector-python from PyPI, for compatibility
# The test suite auto-detects which is installed via
# tests/compatibility.IS_UNIVERSAL_DRIVER, so the same suite runs either way.
case "${AUTH_BROWSER_MODE:-universal}" in
    universal)
        echo "=== Installing Python connector (building sf_core from source) ==="
        pip install -e ".[dev,test]"
        ;;
    reference)
        # The legacy driver's loopback redirect server does not set SO_REUSEADDR,
        # so back-to-back authorization-code tests that reuse the fixed redirect
        # port (8001) hit `Errno 98 Address already in use` while the previous
        # socket lingers in TIME_WAIT. Opt into SO_REUSEPORT, which the old driver
        # honours via this env var, so the next test can rebind immediately.
        export SNOWFLAKE_AUTH_SOCKET_REUSE_PORT=true
        # Test deps come from the editable install; swap the connector itself for
        # the legacy PyPI release. Mirrors the `reference` hatch env in pyproject.
        echo "=== Installing reference snowflake-connector-python ==="
        pip install -e ".[dev,test]"
        pip uninstall -y snowflake-connector-python
        pip install "snowflake-connector-python${PYTHON_REFERENCE_DRIVER_VERSION:->=4,<5}"
        ;;
    *)
        echo "ERROR: unknown AUTH_BROWSER_MODE '${AUTH_BROWSER_MODE}'" >&2
        exit 1
        ;;
esac

echo ""
echo "=== Running Python auth browser E2E tests ==="
python3 -m pytest tests/e2e/authentication/ \
    -v -m requires_browser -n 0
