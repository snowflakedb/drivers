#!/bin/bash

set -e

# Build and run ODBC reference tests using Docker
# This script replicates the odbc_tests_reference workflow from rust.yml

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Building Docker image for ODBC reference tests..."
docker build -t odbc-reference-tests "$SCRIPT_DIR"

echo "Running ODBC reference tests in Docker container..."
docker run --rm \
    -v "$PROJECT_ROOT":/workspace \
    -w /workspace \
    -e DRIVER_PATH="/usr/lib/snowflake/odbc/lib/libSnowflake.so" \
    -e PARAMETER_PATH="/workspace/parameters.json" \
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
        
        echo 'Building and running ODBC tests...'
        cd /workspace/odbc_tests/
        
        # Create build directory
        mkdir -p cmake-build-reference
        
        # Configure CMake
        cmake -B cmake-build-reference \\
            -D ODBC_LIBRARY='/usr/lib/aarch64-linux-gnu/libodbc.so' \\
            -D ODBC_INCLUDE_DIR='/usr/include' \\
            -D SKIP_NEW_TESTS=TRUE \\
            .
        
        # Build tests
        cmake --build cmake-build-reference -- -j \$(nproc)
        
        # Run tests
        echo 'Running ODBC reference tests...'
        ctest -C Debug --test-dir cmake-build-reference --output-on-failure \"\$@\"
    " -- "$@"

echo "ODBC reference tests completed!"
