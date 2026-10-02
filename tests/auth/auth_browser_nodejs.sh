#!/bin/bash
#
# Node.js authentication E2E tests using the shared preprod auth container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

case "${AUTH_BROWSER_MODE:-universal}" in
    universal)
        NPM_SCRIPT=test:e2e
        ;;
    reference)
        NPM_SCRIPT=test:e2e-old-driver
        ;;
    *)
        echo "ERROR: unknown AUTH_BROWSER_MODE '${AUTH_BROWSER_MODE:-}'" >&2
        exit 1
        ;;
esac

# Extra arguments go to vitest as-is: a path relative to nodejs/, plus any flags
# such as -t <name>. Without them the auth directory is the file filter.
# --tags-filter always applies, so untagged tests (JWT, WIF) stay on the normal
# e2e job, as Python, JDBC, and ODBC select only requires_browser here.
if [ "$#" -eq 0 ]; then
    set -- tests/e2e/authentication/
fi

cd "${WORKSPACE_ROOT}/nodejs"

echo "=== Installing Node.js dependencies ==="
npm install

echo ""
echo "=== Running Node.js authentication E2E tests (${NPM_SCRIPT} --tags-filter requires_auth_test_container $*) ==="
npm run "${NPM_SCRIPT}" -- --tags-filter requires_auth_test_container "$@"
