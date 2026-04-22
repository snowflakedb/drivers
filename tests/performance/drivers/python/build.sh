#!/bin/bash
set -e

# Auto-detect architecture if BUILDPLATFORM not set
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building Python performance drivers..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

# Cleanup function
cleanup() {
  rm -rf tests/performance/.tmp
}
trap cleanup EXIT INT TERM

# Create temp directory
mkdir -p tests/performance/.tmp

# Step 1: Build sf-core-builder (shared with ODBC — builds libsf_core.so)
echo "→ Building sf-core-builder..."
docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder ready"
echo ""

# Step 2: Extract libsf_core.so from the builder image
echo "→ Extracting libsf_core.so from sf-core-builder..."
docker rm -f sf-core-extract >/dev/null 2>&1 || true
docker create --name sf-core-extract sf-core-builder:latest >/dev/null 2>&1
if docker cp sf-core-extract:/workdir/libsf_core.so tests/performance/.tmp/libsf_core.so 2>/dev/null; then
    echo "✓ Extracted libsf_core.so"
else
    echo "Error: Could not extract libsf_core.so"
    docker rm -f sf-core-extract >/dev/null 2>&1
    exit 1
fi
docker rm -f sf-core-extract >/dev/null 2>&1

# Get Rust version from sf-core-builder (the version that actually built libsf_core.so)
RUST_VERSION=$(docker run --rm sf-core-builder:latest rustc --version 2>/dev/null | awk '{print $2}' | cut -d. -f1,2 || echo "NA")
echo "${RUST_VERSION}" > tests/performance/.tmp/rust_version
echo "✓ Rust version: ${RUST_VERSION}"
echo ""

# Step 3: Build universal driver image (uses pre-built libsf_core.so, skips cargo build)
echo "→ Building universal driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target universal \
  -t python-perf-driver-universal:latest .

echo ""
echo "✓ Built: python-perf-driver-universal:latest"
echo ""

# Step 4: Build old driver image
echo "→ Building old driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target old \
  -t python-perf-driver-old:latest .

echo ""
echo "✓ Built: python-perf-driver-old:latest"
