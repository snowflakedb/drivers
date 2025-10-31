#!/bin/bash
set -e

BUILDPLATFORM=${BUILDPLATFORM:-linux/amd64}

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building Python performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

# Cleanup function
cleanup() {
  rm -f tests/performance/.tmp_libsf_core.so
}
trap cleanup EXIT INT TERM

# Step 1: Build sf-core-builder
echo "→ Building sf-core-builder..."
docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder ready"
echo ""

# Step 2: Extract libsf_core.so from the builder image to tests/performance/
echo "→ Extracting libsf_core.so from sf-core-builder..."
docker rm -f sf-core-extract >/dev/null 2>&1 || true
docker create --name sf-core-extract sf-core-builder:latest >/dev/null 2>&1
if docker cp sf-core-extract:/workdir/libsf_core.so tests/performance/.tmp_libsf_core.so 2>/dev/null; then
    echo "✓ Extracted libsf_core.so"
else
    echo "❌ Error: Could not extract libsf_core.so"
    docker rm -f sf-core-extract >/dev/null 2>&1
    exit 1
fi
docker rm -f sf-core-extract >/dev/null 2>&1
echo ""

# Step 3: Build Python driver
echo "→ Building Python driver..."
echo ""
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t python-perf-driver:latest .

echo ""
echo "✓ Build complete: python-perf-driver:latest"
