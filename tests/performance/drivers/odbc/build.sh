#!/bin/bash
set -e

# Auto-detect architecture if BUILDPLATFORM not set
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building ODBC performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

# Cleanup function
cleanup() {
  rm -rf tests/performance/.tmp
}
trap cleanup EXIT INT TERM

# Create temp directory
mkdir -p tests/performance/.tmp

# Step 1: Build sf-core-builder
echo "→ Building sf-core-builder (includes sf_core + ODBC wrapper)..."
docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder ready"
echo ""

# Step 2: Extract libsfodbc.so and libsf_core.so from the builder image
echo "→ Extracting libsfodbc.so and libsf_core.so from sf-core-builder..."
docker rm -f sf-core-extract >/dev/null 2>&1 || true
docker create --name sf-core-extract sf-core-builder:latest >/dev/null 2>&1
if docker cp sf-core-extract:/workdir/libsfodbc.so tests/performance/.tmp/libsfodbc.so 2>/dev/null; then
    echo "✓ Extracted libsfodbc.so"
else
    echo "❌ Error: Could not extract libsfodbc.so"
    docker rm -f sf-core-extract >/dev/null 2>&1
    exit 1
fi
if docker cp sf-core-extract:/workdir/libsf_core.so tests/performance/.tmp/libsf_core.so 2>/dev/null; then
    echo "✓ Extracted libsf_core.so"
else
    echo "❌ Error: Could not extract libsf_core.so"
    docker rm -f sf-core-extract >/dev/null 2>&1
    exit 1
fi
docker rm -f sf-core-extract >/dev/null 2>&1

# Get Rust version from sf-core-builder
RUST_VERSION=$(docker run --rm sf-core-builder:latest rustc --version 2>/dev/null | awk '{print $2}' | cut -d. -f1,2 || echo "NA")
echo "${RUST_VERSION}" > tests/performance/.tmp/rust_version
echo "✓ Rust version: ${RUST_VERSION}"
echo ""

# Step 3: Build ODBC driver
echo "→ Building ODBC driver..."
echo ""
docker build -f tests/performance/drivers/odbc/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t odbc-perf-driver:latest .

echo ""
echo "✓ Build complete: odbc-perf-driver:latest"
