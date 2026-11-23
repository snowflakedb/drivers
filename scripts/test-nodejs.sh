#!/usr/bin/env bash
# Run a Node.js Universal Driver Vitest project. Vitest global setup builds the native core.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/test-nodejs.sh <suite> [-- vitest args]

Suites (run as `npm run test:<suite>` in nodejs/):
  unit             unit tests                           no credentials
  types            type tests                           no credentials
  e2e              e2e tests against the new driver     needs parameters.json and a JRE 11+ (WireMock)
  e2e-old-driver   same e2e tests against snowflake-sdk needs parameters.json

Runs `npm install` first when nodejs/node_modules is missing (through npm@11
when the local npm is older, since npm 10 crashes on this install).

Examples:
  scripts/test-nodejs.sh unit
  scripts/test-nodejs.sh e2e -- tests/e2e/connection.test.ts
EOF
}

SUITE=""
VITEST_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --) shift; VITEST_ARGS=("$@"); break ;;
        -*) die "unknown option $1" ;;
        *) [[ -z "$SUITE" ]] || die "unexpected argument $1"; SUITE="$1" ;;
    esac
    shift
done
check_suite usage "$SUITE" unit types e2e e2e-old-driver

require_tool npm "install Node.js 22 or later"
ensure_cargo
if [[ "$SUITE" == e2e* ]]; then
    require_parameters
fi

cd "$REPO_ROOT/nodejs"

if [[ ! -f node_modules/.package-lock.json ]]; then
    NPM_MAJOR="$(npm --version)"
    NPM_MAJOR="${NPM_MAJOR%%.*}"
    if [[ "$NPM_MAJOR" -lt 11 ]]; then
        # npm 10 crashes on this install with "Cannot read properties of null (reading 'edgesOut')".
        run npx --yes npm@11 install
    else
        run npm install
    fi
fi

if [[ ${#VITEST_ARGS[@]} -gt 0 ]]; then
    run npm run "test:$SUITE" -- "${VITEST_ARGS[@]}"
else
    run npm run "test:$SUITE"
fi
