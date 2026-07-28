#!/bin/bash
# ODBC requires_no_mfa tests (parameters_aws_local.json).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_no_mfa_common.sh"

CCACHE_ARGS=""
if command -v ccache &>/dev/null; then
    CCACHE_ARGS="-DCMAKE_CXX_COMPILER_LAUNCHER=ccache -DCMAKE_C_COMPILER_LAUNCHER=ccache"
fi

ODBC_ARGS=()
if command -v odbc_config &>/dev/null; then
    ODBC_ARGS=(
        -D ODBC_LIBRARY="$(odbc_config --lib-prefix)/libodbc.so"
        -D ODBC_INCLUDE_DIR="$(odbc_config --include-prefix)"
    )
fi

echo "=== Building ODBC driver (libsfodbc) ==="
cd "${WORKSPACE_ROOT}"
cargo build
export DRIVER_PATH="${CARGO_TARGET_DIR}/debug/libsfodbc.so"

echo "=== Building ODBC requires_no_mfa tests ==="
cd "${WORKSPACE_ROOT}/odbc_tests"
BUILD_DIR=cmake-build
if [ ! -d "${BUILD_DIR}" ]; then
    mkdir -p "${BUILD_DIR}"
    cmake -B "${BUILD_DIR}" \
        -DCMAKE_CXX_FLAGS="-O0" \
        -DCMAKE_BUILD_TYPE=Debug \
        "${ODBC_ARGS[@]}" \
        -D DRIVER_TYPE=NEW \
        ${CCACHE_ARGS} \
        .
fi
cmake --build "${BUILD_DIR}" --target e2e_auth_no_mfa -- -j 16

echo ""
echo "=== Running ODBC requires_no_mfa tests ==="
ctest -C Debug --test-dir "${BUILD_DIR}" --output-on-failure -L requires_no_mfa
