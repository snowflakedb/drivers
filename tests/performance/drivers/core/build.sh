#!/bin/bash
set -e

# Build with: cd tests/performance && hatch run build-core

BUILDPLATFORM=${BUILDPLATFORM:-linux/amd64}

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building Core performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/core/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t core-perf-driver:latest .

echo ""
echo "✓ Build complete: core-perf-driver:latest"
