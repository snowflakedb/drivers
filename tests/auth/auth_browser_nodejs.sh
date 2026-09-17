#!/bin/bash
#
# Node.js authentication E2E tests using the shared preprod auth container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

case "${AUTH_BROWSER_MODE:-universal}" in
    universal)
        VITEST_PROJECT=e2e
        ;;
    reference)
        VITEST_PROJECT=e2e-old-driver
        ;;
    *)
        echo "ERROR: unknown AUTH_BROWSER_MODE '${AUTH_BROWSER_MODE:-}'" >&2
        exit 1
        ;;
esac

cd "${WORKSPACE_ROOT}/nodejs"

echo "=== Installing Node.js dependencies ==="
npm install

echo ""
echo "=== Running Node.js authentication E2E tests ==="
npx vitest run --project "${VITEST_PROJECT}" tests/e2e/authentication/
