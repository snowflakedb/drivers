#!/bin/bash
# Exit on any error
set -euxo pipefail

source ./scripts/version.sh

echo "=== Platform: $PLATFORM ==="

case "$PLATFORM" in
    linux-x86_64-glibc)
        PLATFORM_TARGET="x86_64-unknown-linux-gnu"
        ;;
    linux-aarch64-glibc)
        PLATFORM_TARGET="aarch64-unknown-linux-gnu"
        ;;
    linux-x86_64-musl)
        PLATFORM_TARGET="x86_64-unknown-linux-musl"
        ;;
    linux-aarch64-musl)
        PLATFORM_TARGET="aarch64-unknown-linux-musl"
        ;;
    macos-x86_64)
        PLATFORM_TARGET="x86_64-apple-darwin"
        ;;
    macos-aarch64)
        PLATFORM_TARGET="aarch64-apple-darwin"
        ;;
    windows-aarch64)
        PLATFORM_TARGET="aarch64-pc-windows-msvc"
        ;;
    windows-x86_64)
        PLATFORM_TARGET="x86_64-pc-windows-msvc"
        ;;
    aix-ppc64)
        PLATFORM_TARGET="powerpc64-ibm-aix"
        ;;
    *)
        echo "Unknown platform: $PLATFORM"
        exit 1
        ;;
esac

CARGO_CMD="cargo"
if [[ "$PLATFORM" == windows-* ]]; then
    CARGO_CMD="cargo xwin"
fi

echo "=== Platform target: $PLATFORM_TARGET ==="
echo "=== Ensuring target is installed ==="
# Should be already installed on AIX
if [[ ! "$PLATFORM" == aix-* ]]; then
    rustup target add $PLATFORM_TARGET
fi

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
$CARGO_CMD build --release --package sf_mini_core --target $PLATFORM_TARGET 

echo "=== Building static library version ==="
$CARGO_CMD build --release --package sf_mini_core_static --target $PLATFORM_TARGET

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
    windows-*)
        DYLIB_EXT="dll"
        ;;
    *)
        echo "Unknown platform: $PLATFORM"
        exit 1
        ;;
esac
DYLIB_NAME=libsf_mini_core.$DYLIB_EXT

echo "=== Compiling library artifact list ==="
case "$PLATFORM" in
    windows-*)
        LIB_LIST="sf_mini_core_static.lib sf_mini_core.dll"
        ;;
    *)
        LIB_LIST="libsf_mini_core_static.a $DYLIB_NAME"
        ;;
esac
echo "=== Library artifact list: $LIB_LIST ==="

# Copy build artifacts
echo "=== Copying library artifacts ==="
for library in $LIB_LIST; do
    echo "=== Copying library: $library ==="
    cp target/$PLATFORM_TARGET/release/$library $BUILD_DIR/
done

PACKAGE_NAME=sf_mini_core_${PLATFORM}_${VERSION}_SNAPSHOT_${COMMIT_SHA}.tar.gz

# Create archive
echo "=== Creating archive: $PACKAGE_NAME ==="
ARTIFACT_LIST="sf_mini_core.h $LIB_LIST"
pushd $BUILD_DIR
    tar -cvf - $ARTIFACT_LIST | gzip > $PACKAGE_NAME
popd > /dev/null

echo "=== Successfully created archive at $BUILD_DIR/$PACKAGE_NAME ==="

