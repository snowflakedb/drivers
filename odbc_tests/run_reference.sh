#!/bin/bash

set -e

# Build and run ODBC reference tests using Docker
# This script replicates the odbc_tests_reference workflow from rust.yml

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

REFERENCE_ODBC_VERSION=$(cat "$PROJECT_ROOT/ci/reference-odbc-version" | tr -d '[:space:]')
. "$PROJECT_ROOT/ci/reference-odbc-checksums"

HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" = "x86_64" ]; then
    REFERENCE_ODBC_SHA256="$DEB_X86_64_SHA256"
else
    REFERENCE_ODBC_SHA256="$DEB_AARCH64_SHA256"
fi

echo "Using reference ODBC driver version: $REFERENCE_ODBC_VERSION (${HOST_ARCH})"

echo "Building Docker image for ODBC reference tests..."
docker build \
    --build-arg REFERENCE_ODBC_VERSION="$REFERENCE_ODBC_VERSION" \
    --build-arg REFERENCE_ODBC_SHA256="$REFERENCE_ODBC_SHA256" \
    -t odbc-reference-tests "$SCRIPT_DIR"

echo "Running ODBC reference tests in Docker container..."
docker run --rm \
    -v "$PROJECT_ROOT":/workspace \
    -w /workspace \
    -e DRIVER_PATH="/usr/lib/snowflake/odbc/lib/libSnowflake.so" \
    -e PARAMETER_PATH="/workspace/parameters.json" \
    -e GIT_ROOT="/workspace" \
    odbc-reference-tests \
    bash -c "
        set -e
        set -x
        echo 'Setting up test environment...'
        
        # Decode secrets (parameters.json should be mounted)
        if [ ! -f /workspace/parameters.json ]; then
            echo 'Error: parameters.json not found. Please run ./scripts/decode_secrets.sh first.'
            exit 1
        fi
        
        # Detect ODBC library path for the container architecture
        ODBC_LIB=\$(find /usr/lib -name 'libodbc.so' -print -quit)
        
        echo 'Building and running ODBC tests...'
        cd /workspace/odbc_tests/
        
        # Create build directory
        mkdir -p cmake-build-reference
        
        # Configure CMake
        cmake -B cmake-build-reference \\
            -D ODBC_LIBRARY=\"\$ODBC_LIB\" \\
            -D ODBC_INCLUDE_DIR='/usr/include' \\
            -D DRIVER_TYPE=OLD \\
            .
        
        # Build tests
        cmake --build cmake-build-reference -- -j \$(nproc)
        
        # Run tests
        echo 'Running ODBC reference tests...'
        ctest -j \$(nproc) -C Debug --test-dir cmake-build-reference --output-on-failure \"\$@\"
    " -- "$@"

echo "ODBC reference tests completed!"
