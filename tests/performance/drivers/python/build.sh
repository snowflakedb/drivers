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

echo "→ Building universal driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target universal \
  -t python-perf-driver-universal:latest .

echo ""
echo "✓ Built: python-perf-driver-universal:latest"
echo ""

echo "→ Building old driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target old \
  -t python-perf-driver-old:latest .

echo ""
echo "✓ Built: python-perf-driver-old:latest"
