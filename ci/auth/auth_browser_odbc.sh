#!/bin/bash
#
# ODBC authentication E2E tests requiring a headless browser.
# Runs inside the snowdrivers-test-external-browser-universal-driver Docker image.
#
# Local usage (requires parameters.json in repo root):
#
#   docker run --rm --platform linux/amd64 \
#     -v "$PWD:/mnt/host" -e WORKSPACE_ROOT=/mnt/host \
#     snowdrivers-test-external-browser-universal-driver:1 \
#     bash /mnt/host/ci/auth/auth_browser_odbc.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

# TODO: fold these into the snowdrivers-test-external-browser-universal-driver image.
# Today the image ships Rust + Python + Node + Chromium but NOT the C++/ODBC build
# toolchain, so install it at runtime (the container runs as root on Debian 12).
echo "=== Installing ODBC build dependencies ==="
if ! command -v odbc_config &>/dev/null; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends \
        unixodbc unixodbc-dev odbcinst build-essential wget ca-certificates
fi

# The test CMakeLists requires CMake >= 4.0, newer than Debian's apt package, so
# install the official Kitware build (mirrors odbc_tests/Dockerfile).
if ! command -v cmake &>/dev/null || ! cmake --version | grep -qiE 'version (4|[5-9])\.'; then
    CMAKE_VERSION=4.0.3
    ARCH=$(dpkg --print-architecture)
    if [ "$ARCH" = "amd64" ]; then CMAKE_ARCH="x86_64"; else CMAKE_ARCH="aarch64"; fi
    CMAKE_INSTALLER="cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}.sh"
    wget -q "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/${CMAKE_INSTALLER}"
    sh "${CMAKE_INSTALLER}" --skip-license --prefix=/usr/local
    rm -f "${CMAKE_INSTALLER}"
fi
cmake --version

echo "=== Building ODBC driver (libsfodbc) ==="
cd "${WORKSPACE_ROOT}"
cargo build
# CARGO_TARGET_DIR is exported by auth_browser_common.sh.
export DRIVER_PATH="${CARGO_TARGET_DIR}/debug/libsfodbc.so"

echo "=== Configuring & building ODBC e2e external browser test ==="
CCACHE_ARGS=""
if command -v ccache &>/dev/null; then
    CCACHE_ARGS="-DCMAKE_CXX_COMPILER_LAUNCHER=ccache -DCMAKE_C_COMPILER_LAUNCHER=ccache"
fi

cd "${WORKSPACE_ROOT}/odbc_tests"
if [ ! -d cmake-build ]; then
    mkdir -p cmake-build
    # Point CMake at unixODBC via odbc_config when available (e.g. macOS/Homebrew,
    # Ubuntu CI). Debian's unixodbc-dev does not ship odbc_config, but it installs
    # the headers and libodbc.so in standard paths, so CMake's FindODBC auto-detects.
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
cmake --build cmake-build --target e2e_authentication_external_browser -- -j 16

echo ""
echo "=== Running ODBC auth browser E2E tests ==="
ctest -C Debug --test-dir cmake-build --output-on-failure -R e2e_authentication_external_browser
