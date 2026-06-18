#!/bin/bash
#
# ODBC authentication E2E tests requiring a headless browser container.
# Runs the external-browser (headless Chromium) suite inside
# snowdrivers-test-external-browser-universal-driver.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

echo "=== Building ODBC driver (libsfodbc) ==="
cd "${WORKSPACE_ROOT}"
cargo build
export DRIVER_PATH="${CARGO_TARGET_DIR}/debug/libsfodbc.so"

echo "=== Configuring & building ODBC e2e auth browser tests ==="
CCACHE_ARGS=""
if command -v ccache &>/dev/null; then
    CCACHE_ARGS="-DCMAKE_CXX_COMPILER_LAUNCHER=ccache -DCMAKE_C_COMPILER_LAUNCHER=ccache"
fi

cd "${WORKSPACE_ROOT}/odbc_tests"
if [ ! -d cmake-build ]; then
    mkdir -p cmake-build
    ODBC_ARGS=()
    if command -v odbc_config &>/dev/null; then
        ODBC_ARGS=(
            -D ODBC_LIBRARY="$(odbc_config --lib-prefix)/libodbc.so"
            -D ODBC_INCLUDE_DIR="$(odbc_config --include-prefix)"
        )
    fi
    cmake -B cmake-build \
        -DCMAKE_CXX_FLAGS="-O0" \
        -DCMAKE_BUILD_TYPE=Debug \
        "${ODBC_ARGS[@]}" \
        -D DRIVER_TYPE=NEW \
        ${CCACHE_ARGS} \
        .
fi
cmake --build cmake-build --target e2e_auth_browser -- -j 16

echo ""
echo "=== Running ODBC auth browser E2E tests ==="
# Select by the shared `requires_browser` Catch2 tag (exposed as a CTest label
# via ADD_TAGS_AS_LABELS) rather than by target/file name, mirroring the
# `requires_browser` pytest marker used by the Python suite.
ctest -C Debug --test-dir cmake-build --output-on-failure -L requires_browser
