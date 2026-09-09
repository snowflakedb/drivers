#!/bin/bash
#
# Node.js authentication E2E tests using the shared preprod auth container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

if [ "${AUTH_BROWSER_MODE:-universal}" != "universal" ]; then
    echo "ERROR: Node.js reference auth tests are not enabled yet" >&2
    exit 1
fi

cd "${WORKSPACE_ROOT}/nodejs"

echo "=== Installing Node.js dependencies ==="
npm install

echo ""
echo "=== Running Node.js legacy OAuth E2E test ==="
npx vitest run --project e2e tests/e2e/authentication/oauth.test.ts
