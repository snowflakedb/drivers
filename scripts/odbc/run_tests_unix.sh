#!/bin/bash
# Run ODBC tests on Unix (Linux/macOS)
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
set -euo pipefail

cd "$(dirname "$0")/../../odbc_tests"

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

mkdir -p cmake-build
cmake -B cmake-build \
    -D ODBC_LIBRARY="${ODBC_LIBRARY}" \
    -D ODBC_INCLUDE_DIR="${ODBC_INCLUDE_DIR}" \
    -D DRIVER_TYPE="${DRIVER_TYPE}" \
    .
cmake --build cmake-build -- -j "${NPROC}"
ctest -j "${NPROC}" -C Debug --test-dir cmake-build --output-on-failure
