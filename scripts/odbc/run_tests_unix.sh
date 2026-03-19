#!/bin/bash
# Run ODBC tests on Unix (Linux/macOS)
#
# Required env vars:
#   DRIVER_PATH       Path to the ODBC driver shared library
#   PARAMETER_PATH    Path to parameters.json
#   DRIVER_TYPE       "NEW" or "OLD"
#
# Optional env vars:
#   FORCE_RUN_NOT_IMPLEMENTED   Set to "ON" to disable SKIP_NEW_DRIVER_NOT_IMPLEMENTED
set -euo pipefail

cd "$(dirname "$0")/../../odbc_tests"

BUILD_DIR="${1:-cmake-build}"
shift 2>/dev/null || true

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

CMAKE_EXTRA_ARGS=()
if [[ "${FORCE_RUN_NOT_IMPLEMENTED:-}" == "ON" ]]; then
    CMAKE_EXTRA_ARGS+=(-D FORCE_RUN_NOT_IMPLEMENTED=ON)
fi

mkdir -p "$BUILD_DIR"
cmake -B "$BUILD_DIR" \
    -D ODBC_LIBRARY="${ODBC_LIBRARY}" \
    -D ODBC_INCLUDE_DIR="${ODBC_INCLUDE_DIR}" \
    -D DRIVER_TYPE="${DRIVER_TYPE}" \
    "${CMAKE_EXTRA_ARGS[@]+"${CMAKE_EXTRA_ARGS[@]}"}" \
    .
cmake --build "$BUILD_DIR" -- -j $((NPROC * 2))
ctest -j $((NPROC * 4)) -C Debug --test-dir "$BUILD_DIR" --output-on-failure --output-junit results.xml "$@"
