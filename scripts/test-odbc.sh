#!/usr/bin/env bash
# Run ODBC Universal Driver Rust unit tests, or build libsfodbc and run the C++ ctest harness.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/test-odbc.sh <suite> [--iodbc] [-- runner args]

Suites:
  unit        cargo nextest (or cargo test) on the odbc crate    no credentials
  e2e         build libsfodbc, then scripts/odbc/run_tests_unix.sh
              (cmake + ctest in odbc_tests/)                     needs parameters.json
  reference   odbc_tests/run_reference.sh: the same ctest suite
              against the old Snowflake ODBC driver, in Docker   needs parameters.json

Options:
  --iodbc     e2e only: build and run against iODBC instead of unixODBC

Runner args go to nextest/cargo test (unit) or ctest (e2e, reference).

Examples:
  scripts/test-odbc.sh unit
  scripts/test-odbc.sh e2e -- -R 'connection'
  scripts/test-odbc.sh e2e -- -L long_running
EOF
}

SUITE=""
DRIVER_MANAGER=unixodbc
RUNNER_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --iodbc) DRIVER_MANAGER=iodbc ;;
        --) shift; RUNNER_ARGS=("$@"); break ;;
        -*) die "unknown option $1" ;;
        *) [[ -z "$SUITE" ]] || die "unexpected argument $1"; SUITE="$1" ;;
    esac
    shift
done
check_suite usage "$SUITE" unit e2e reference

cd "$REPO_ROOT"

case "$SUITE" in
    unit)
        ensure_cargo
        if [[ "$(uname -s)" == Linux ]]; then
            export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,--undefined-version"
        fi
        if cargo nextest --version >/dev/null 2>&1; then
            run cargo nextest run --locked --package odbc --no-fail-fast ${RUNNER_ARGS[@]+"${RUNNER_ARGS[@]}"}
        else
            echo "cargo-nextest not installed; falling back to cargo test (no per-test retries)."
            run cargo test --locked --package odbc ${RUNNER_ARGS[@]+"${RUNNER_ARGS[@]}"}
        fi
        ;;
    e2e)
        ensure_cargo
        require_tool cmake
        require_parameters
        run cargo build --locked --package odbc
        DRIVER_PATH="$(cargo_target_dir)/debug/$(native_lib_name sfodbc)"
        export DRIVER_PATH
        [[ -f "$DRIVER_PATH" ]] || die "expected $DRIVER_PATH after cargo build"
        export DRIVER_TYPE=NEW
        export DRIVER_MANAGER
        run ./scripts/odbc/run_tests_unix.sh ${RUNNER_ARGS[@]+"${RUNNER_ARGS[@]}"}
        ;;
    reference)
        require_tool docker
        require_parameters
        [[ "$PARAMETER_PATH" == "$REPO_ROOT/parameters.json" ]] \
            || die "reference mounts the repo into Docker and reads ./parameters.json; PARAMETER_PATH is ignored there"
        run ./odbc_tests/run_reference.sh ${RUNNER_ARGS[@]+"${RUNNER_ARGS[@]}"}
        ;;
esac
