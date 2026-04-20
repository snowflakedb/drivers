#!/bin/bash
# Run ODBC tests on Unix (Linux/macOS)
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/../../odbc_tests"

# Detect ODBC paths
if [[ "$(uname)" == "Darwin" ]]; then
    ODBC_PREFIX=$(brew --prefix unixodbc)
    ODBC_LIBRARY="${ODBC_PREFIX}/lib/libodbc.dylib"
    ODBC_INCLUDE_DIR="${ODBC_PREFIX}/include"
    NPROC=$(sysctl -n hw.ncpu)
else
    ODBC_LIBRARY="/usr/lib/x86_64-linux-gnu/libodbc.so"
    ODBC_INCLUDE_DIR="/usr/include"
    NPROC=$(nproc)
fi

CCACHE_ARGS=""
if command -v ccache &>/dev/null; then
    CCACHE_ARGS="-DCMAKE_CXX_COMPILER_LAUNCHER=ccache -DCMAKE_C_COMPILER_LAUNCHER=ccache"
fi

# Prefer Ninja over the default Unix Makefiles generator: file-level parallelism and
# faster dependency scanning typically save 20-40s on the C++ test-harness build.
# Ninja is pre-installed on both ubuntu-latest and macos-latest GHA runners; local devs
# may not have it, so gracefully fall back to the default generator if unavailable.
GENERATOR_ARGS=()
if command -v ninja &>/dev/null; then
    GENERATOR_ARGS=(-G Ninja -DCMAKE_BUILD_TYPE=Debug)
fi

mkdir -p cmake-build
cmake -B cmake-build \
    "${GENERATOR_ARGS[@]}" \
    -D ODBC_LIBRARY="${ODBC_LIBRARY}" \
    -D ODBC_INCLUDE_DIR="${ODBC_INCLUDE_DIR}" \
    -D DRIVER_TYPE="${DRIVER_TYPE}" \
    ${CCACHE_ARGS} \
    .
cmake --build cmake-build -- -j $((NPROC * 2))

# --- Schema lifecycle: pre-create a shared schema for all test processes ----------
SCHEMA_TOOL="./cmake-build/tools/schema_tool"
if SCHEMA_NAME=$("$SCHEMA_TOOL" create); then
    if [[ ! "$SCHEMA_NAME" =~ ^TEMP_TEST_SCHEMA_[0-9]+$ ]]; then
        echo "run_tests: schema_tool returned invalid name '$SCHEMA_NAME', falling back to per-process"
    else
        export ODBC_TEST_SCHEMA="$SCHEMA_NAME"
        trap '"$SCHEMA_TOOL" drop "$SCHEMA_NAME" 2>/dev/null || true' EXIT
        echo "run_tests: using shared schema $SCHEMA_NAME"
    fi
else
    echo "run_tests: schema pre-creation failed, falling back to per-process"
fi

CTEST_ARGS=()
if [[ -n "${CTEST_FILTER:-}" ]]; then
    CTEST_ARGS+=(-R "$CTEST_FILTER")
fi
ctest -j $((NPROC * 4)) -C Debug --test-dir cmake-build --output-on-failure ${CTEST_ARGS[@]+"${CTEST_ARGS[@]}"} "$@"
