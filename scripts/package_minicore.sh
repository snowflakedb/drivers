#!/bin/bash
# Exit on any error
set -euxo pipefail

source ./scripts/version.sh

echo "=== Platform: $PLATFORM ==="
echo "=== Commit SHA: $COMMIT_SHA ==="

echo "=== Checking if cbindgen is installed ==="
# Install cbindgen if not available
if ! command -v cbindgen &> /dev/null; then
    echo "=== Installing cbindgen ==="
    cargo install cbindgen
    # Ensure cargo bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# Set build directory
BUILD_DIR=$(pwd)/build
echo "=== Using build directory: $BUILD_DIR ==="

# Create build directory if it doesn't exist
echo "=== Creating build directory: $BUILD_DIR ==="
mkdir -p $BUILD_DIR

# Generate C header file
echo "=== Generating C header file: $BUILD_DIR/sf_mini_core.h ==="
cbindgen --config sf_mini_core/cbindgen.toml --crate sf_mini_core > $BUILD_DIR/sf_mini_core.h

# Build release version
echo "=== Building dynamic library version ==="
cargo build --release --package sf_mini_core

echo "=== Building static library version ==="
cargo build --release --package sf_mini_core_static

# Determine dynamic library extension based on platform
case "$PLATFORM" in
    linux-*)
        DYLIB_EXT="so"
        ;;
    macos-*)
        DYLIB_EXT="dylib"
        ;;
    aix-*)
        DYLIB_EXT="a"
        ;;
    *)
        echo "Unknown platform: $PLATFORM"
        exit 1
        ;;
esac
DYLIB_NAME=libsf_mini_core.$DYLIB_EXT

# Copy build artifacts
echo "=== Copying build artifacts ==="
# Copy static library
cp target/release/libsf_mini_core_static.a $BUILD_DIR/
# Copy dynamic library
cp target/release/$DYLIB_NAME $BUILD_DIR/

PACKAGE_NAME=sf_mini_core_${PLATFORM}_${VERSION}_SNAPSHOT_${COMMIT_SHA}.tar.gz

# Create archive
echo "=== Creating archive: $PACKAGE_NAME ==="
pushd $BUILD_DIR
    tar -cvf - sf_mini_core.h libsf_mini_core_static.a $DYLIB_NAME | gzip > $PACKAGE_NAME
popd > /dev/null

echo "=== Successfully created archive at $BUILD_DIR/$PACKAGE_NAME ==="

