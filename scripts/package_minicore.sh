# Exit on any error
set -e

# Print commands as they are executed
set -x

# Set build directory
BUILD_DIR=$(pwd)/build
echo "=== Using build directory: $BUILD_DIR"

# Create build directory if it doesn't exist
echo "=== Creating build directory..."
mkdir -p $BUILD_DIR

# Generate C header file
echo "=== Generating C header file..."
cbindgen --config sf_mini_core/cbindgen.toml --crate sf_mini_core > $BUILD_DIR/sf_mini_core.h

# Build release version
echo "=== Building release version..."
cargo build --release --package sf_mini_core

# Copy build artifacts
echo "=== Copying build artifacts..."
# Copy static library
cp target/release/libsf_mini_core.a $BUILD_DIR/
# Copy dynamic library
cp target/release/libsf_mini_core.dylib $BUILD_DIR/

# Create archive
echo "=== Creating archive..."
cd $BUILD_DIR
tar -czf sf_mini_core.tar.gz sf_mini_core.h libsf_mini_core.a libsf_mini_core.dylib
cd - > /dev/null

echo "=== Successfully created archive at $BUILD_DIR/sf_mini_core.tar.gz"
