#!/bin/bash
#
# ODBC authentication E2E tests requiring a headless browser container.
# Runs the external-browser (headless Chromium) suite inside
# snowdrivers-test-external-browser-universal-driver.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

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

# AUTH_BROWSER_MODE selects which driver to test (default: universal/new).
#   universal -> build libsfodbc from source, DRIVER_TYPE=NEW
#   reference -> install legacy Snowflake ODBC .deb, DRIVER_TYPE=OLD
# Each mode uses its own cmake build dir so the two can coexist on a reused
# workspace. Mirrors odbc_tests/Dockerfile + odbc_tests/run_reference.sh.
case "${AUTH_BROWSER_MODE:-universal}" in
    universal)
        echo "=== Building ODBC driver (libsfodbc) ==="
        cd "${WORKSPACE_ROOT}"
        cargo build --package odbc
        export DRIVER_PATH="${CARGO_TARGET_DIR}/debug/libsfodbc.so"
        DRIVER_TYPE=NEW
        BUILD_DIR=cmake-build
        ;;
    reference)
        echo "=== Installing reference Snowflake ODBC driver ==="
        REFERENCE_ODBC_VERSION="$(tr -d '[:space:]' < "${WORKSPACE_ROOT}/ci/reference-odbc-version")"
        # shellcheck disable=SC1091
        . "${WORKSPACE_ROOT}/ci/reference-odbc-checksums"
        DPKG_ARCH="$(dpkg --print-architecture)"
        if [ "$DPKG_ARCH" = "amd64" ]; then
            PKG_ARCH="x86_64"; URL_PATH="linux"; REFERENCE_ODBC_SHA256="$DEB_X86_64_SHA256"
        else
            PKG_ARCH="aarch64"; URL_PATH="linuxaarch64"; REFERENCE_ODBC_SHA256="$DEB_AARCH64_SHA256"
        fi
        DEB="snowflake-odbc-${REFERENCE_ODBC_VERSION}.${PKG_ARCH}.deb"
        echo "Using reference ODBC driver version: ${REFERENCE_ODBC_VERSION} (${PKG_ARCH})"
        cd "${WORKSPACE_ROOT}"
        curl -fsSLO "https://sfc-repo.snowflakecomputing.com/odbc/${URL_PATH}/${REFERENCE_ODBC_VERSION}/${DEB}"
        echo "${REFERENCE_ODBC_SHA256}  ${DEB}" | sha256sum -c -
        dpkg -i "${DEB}" || apt-get install -y -f
        rm -f "${DEB}"
        ODBC_INST_DIR="$(find /usr/lib -name 'libodbcinst.so.2' -printf '%h' -quit)"
        sed -i "s,ODBCInstLib=.*,ODBCInstLib=${ODBC_INST_DIR}/libodbcinst.so.2,g" \
            /usr/lib/snowflake/odbc/lib/simba.snowflake.ini
        export DRIVER_PATH="/usr/lib/snowflake/odbc/lib/libSnowflake.so"
        DRIVER_TYPE=OLD
        BUILD_DIR=cmake-build-reference

        # The old driver's OCSP code creates $HOME/.cache/snowflake/ with 0755,
        # but its credential cache (libsnowflakeclient CacheFile.cpp) requires
        # exactly 0700 and silently skips the directory otherwise. Pre-create it
        # with the correct permissions so MFA token caching works.
        mkdir -p "${HOME}/.cache/snowflake"
        chmod 700 "${HOME}/.cache/snowflake"
        ;;
    *)
        echo "ERROR: unknown AUTH_BROWSER_MODE '${AUTH_BROWSER_MODE}'" >&2
        exit 1
        ;;
esac

echo "=== Configuring & building ODBC e2e auth browser tests (DRIVER_TYPE=${DRIVER_TYPE}) ==="
cd "${WORKSPACE_ROOT}/odbc_tests"
if [ ! -d "${BUILD_DIR}" ]; then
    mkdir -p "${BUILD_DIR}"
    cmake -B "${BUILD_DIR}" \
        -DCMAKE_CXX_FLAGS="-O0" \
        -DCMAKE_BUILD_TYPE=Debug \
        "${ODBC_ARGS[@]}" \
        -D DRIVER_TYPE="${DRIVER_TYPE}" \
        ${CCACHE_ARGS} \
        .
fi
cmake --build "${BUILD_DIR}" --target e2e_auth_browser -- -j 16

echo ""
echo "=== Running ODBC auth browser E2E tests ==="
# Select by the shared `requires_browser` Catch2 tag (exposed as a CTest label
# via ADD_TAGS_AS_LABELS) rather than by target/file name, mirroring the
# `requires_browser` pytest marker used by the Python suite.
ctest -C Debug --test-dir "${BUILD_DIR}" --output-on-failure -L requires_browser
