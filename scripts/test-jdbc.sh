#!/usr/bin/env bash
# Build jdbc_bridge and run a JDBC Universal Driver Gradle test task.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/test-jdbc.sh <suite> [--json] [-- gradle args]

Suites (all need parameters.json):
  test        ./gradlew test           Universal Driver tests against jdbc_bridge
  reference   ./gradlew referenceTest  same tests against the old snowflake-jdbc (no Rust build)
  parity      ./gradlew parityTest     date/time parity against snowflake-jdbc (needs a HotSpot JDK)

Options:
  --json      Set QUERY_RESULT_FORMAT=JSON (default is Arrow)

Examples:
  scripts/test-jdbc.sh test
  scripts/test-jdbc.sh test -- --tests '*ConnectionStringTest'
EOF
}

SUITE=""
JSON=0
GRADLE_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --json) JSON=1 ;;
        --) shift; GRADLE_ARGS=("$@"); break ;;
        -*) die "unknown option $1" ;;
        *) [[ -z "$SUITE" ]] || die "unexpected argument $1"; SUITE="$1" ;;
    esac
    shift
done
check_suite usage "$SUITE" test reference parity

require_parameters
cd "$REPO_ROOT"

case "$SUITE" in
    test) TASK="test" ;;
    reference) TASK=referenceTest ;;
    parity) TASK=parityTest ;;
esac

if [[ "$SUITE" != reference ]]; then
    ensure_cargo
    run cargo build --locked --package jdbc_bridge
    CORE_PATH="$(cargo_target_dir)/debug/$(native_lib_name jdbc_bridge)"
    export CORE_PATH
    [[ -f "$CORE_PATH" ]] || die "expected $CORE_PATH after cargo build"
fi

if [[ "$JSON" == 1 ]]; then
    export QUERY_RESULT_FORMAT=JSON
fi

cd jdbc
run ./gradlew "$TASK" ${GRADLE_ARGS[@]+"${GRADLE_ARGS[@]}"}
