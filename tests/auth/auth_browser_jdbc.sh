#!/bin/bash
#
# JDBC authentication E2E tests requiring a headless browser container.
# Runs the external-browser (headless Chromium) suite inside
# snowdrivers-test-external-browser-universal-driver.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

# AUTH_BROWSER_MODE selects which driver to test (default: universal/new):
#   universal -> `test`          (this module's driver)
#   reference -> `referenceTest` (legacy snowflake-jdbc, for compatibility)
case "${AUTH_BROWSER_MODE:-universal}" in
    universal) GRADLE_TASK=test ;;
    reference) GRADLE_TASK=referenceTest ;;
    *) echo "ERROR: unknown AUTH_BROWSER_MODE '${AUTH_BROWSER_MODE}'" >&2; exit 1 ;;
esac

echo "=== Building JDBC bridge (libjdbc_bridge) ==="
cd "${WORKSPACE_ROOT}"
cargo build -p jdbc_bridge

echo ""
echo "=== Running JDBC auth browser E2E tests (task: ${GRADLE_TASK}, tag: requires_browser) ==="
cd "${WORKSPACE_ROOT}/jdbc"
GRADLE_INCLUDE_TAGS=requires_browser ./gradlew "${GRADLE_TASK}"
