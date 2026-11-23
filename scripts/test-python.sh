#!/usr/bin/env bash
# Build the Python Universal Driver core if needed and run a pytest suite through Hatch.
set -euo pipefail

# shellcheck source=scripts/lib/test-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/test-common.sh"

usage() {
    cat <<'EOF'
Usage: scripts/test-python.sh <suite> [--py VERSION] [--rebuild-core] [-- pytest args]

Suites (run as `hatch run dev.py<VERSION>:<suite>` in python/):
  unit    tests/unit                       no credentials
  integ   tests/integ                      needs parameters.json
  e2e     tests/e2e                        needs parameters.json
  all     tests/ except tests/packaging    needs parameters.json

Options:
  --py VERSION      Hatch dev matrix Python (default 3.13; e.g. 3.12, 3.14t)
  --rebuild-core    Recreate the Hatch env with SKIP_CORE_BUILD=0 so the Rust
                    core (sf_core_python) is rebuilt. Happens automatically when
                    no core binary for VERSION exists yet.

Examples:
  scripts/test-python.sh unit
  scripts/test-python.sh e2e --py 3.12 -- -k test_connection -x
EOF
}

SUITE=""
PY="3.13"
REBUILD_CORE=0
PYTEST_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --py) PY="${2:?--py needs a value}"; shift ;;
        --rebuild-core) REBUILD_CORE=1 ;;
        --) shift; PYTEST_ARGS=("$@"); break ;;
        -*) die "unknown option $1" ;;
        *) [[ -z "$SUITE" ]] || die "unexpected argument $1"; SUITE="$1" ;;
    esac
    shift
done
check_suite usage "$SUITE" unit integ e2e all

require_tool hatch "install with: uv tool install hatch"
ensure_cargo

ENV="dev.py$PY"
CORE_DIR="$REPO_ROOT/python/src/snowflake/connector/_core"
PY_TAG="${PY/./}"

cd "$REPO_ROOT/python"

core_built() {
    compgen -G "$CORE_DIR/sf_core_python.cpython-$PY_TAG-*" >/dev/null
}

if [[ "$REBUILD_CORE" == 1 ]] || ! core_built; then
    echo "Building sf_core_python for Python $PY (Rust release build; this takes a few minutes)."
    run hatch env remove "$ENV"
    run env SKIP_CORE_BUILD=0 hatch env create "$ENV"
    core_built || die "no sf_core_python binary for Python $PY in $CORE_DIR after the build"
fi

if [[ "$SUITE" != unit ]]; then
    require_parameters
fi

if [[ "$PY" == *t ]]; then
    export PYTHON_GIL=0
fi

run hatch run "$ENV:$SUITE" ${PYTEST_ARGS[@]+"${PYTEST_ARGS[@]}"}
