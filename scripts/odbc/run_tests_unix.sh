#!/bin/bash
# Run ODBC tests on Unix (Linux/macOS)
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
# Optional env vars:
#   DRIVER_MANAGER  one of `unixodbc` (default) | `iodbc`. When `iodbc`, the
#                   harness builds against libiodbc's `<sql.h>` (where
#                   `SQLWCHAR == wchar_t == 4 bytes`) and writes a temporary
#                   `sf.odbc.ini` with `DriverManagerEncoding=UTF-32` so the
#                   universal driver matches.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/../../odbc_tests"

DRIVER_MANAGER="${DRIVER_MANAGER:-unixodbc}"

# Detect ODBC paths for the chosen driver manager. The C++ test harness binds
# `SQLWCHAR` size at compile time from the DM's `<sql.h>`, so picking the
# right include dir here is what flips the test side between UTF-16 and UTF-32.
if [[ "$(uname)" == "Darwin" ]]; then
    if [[ "$DRIVER_MANAGER" == "iodbc" ]]; then
        # macOS ships /usr/lib/libiodbc.dylib but its headers aren't in the
        # public SDK; use Homebrew's libiodbc which provides both.
        DM_PREFIX=$(brew --prefix libiodbc)
        ODBC_LIBRARY="${DM_PREFIX}/lib/libiodbc.dylib"
        ODBC_INCLUDE_DIR="${DM_PREFIX}/include"
    else
        DM_PREFIX=$(brew --prefix unixodbc)
        ODBC_LIBRARY="${DM_PREFIX}/lib/libodbc.dylib"
        ODBC_INCLUDE_DIR="${DM_PREFIX}/include"
    fi
    NPROC=$(sysctl -n hw.ncpu)
else
    if [[ "$DRIVER_MANAGER" == "iodbc" ]]; then
        ODBC_LIBRARY="/usr/lib/x86_64-linux-gnu/libiodbc.so.2"
        ODBC_INCLUDE_DIR="/usr/include/iodbc"
    else
        ODBC_LIBRARY="/usr/lib/x86_64-linux-gnu/libodbc.so"
        ODBC_INCLUDE_DIR="/usr/include"
    fi
    NPROC=$(nproc)
fi

# When testing under iODBC the universal driver must be configured for the
# matching 4-byte SQLWCHAR encoding. Materialise a per-job sf.odbc.ini and
# point SF_ODBC_INI at it; the driver loads this once, on first wide-string
# call, and caches the value for the remainder of the process. We keep the
# file under the test build dir (auto-cleaned with the workspace) and chmod
# it to 0600 so the driver's permission check accepts it.
if [[ "$DRIVER_MANAGER" == "iodbc" ]]; then
    SF_ODBC_INI_FILE="$(pwd)/cmake-build/sf.odbc.ini"
    mkdir -p "$(dirname "$SF_ODBC_INI_FILE")"
    cat > "$SF_ODBC_INI_FILE" <<'EOF'
DriverManagerEncoding=UTF-32
EOF
    chmod 600 "$SF_ODBC_INI_FILE"
    export SF_ODBC_INI="$SF_ODBC_INI_FILE"
    export SF_RUNNING_IODBC_TEST_SUITE=1
    echo "run_tests: configured SF_ODBC_INI=$SF_ODBC_INI_FILE for iODBC (UTF-32)"
    echo "run_tests: exported SF_RUNNING_IODBC_TEST_SUITE=1 for iODBC test gating"
else
    unset SF_RUNNING_IODBC_TEST_SUITE
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
    ${GENERATOR_ARGS[@]+"${GENERATOR_ARGS[@]}"} \
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
