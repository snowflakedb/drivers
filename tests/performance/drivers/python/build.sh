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

# Step 1: Build universal driver image (hatch builds python_bridge for this image's Python)
echo "→ Building universal driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target universal \
  -t python-perf-driver-universal:latest .

echo ""
echo "✓ Built: python-perf-driver-universal:latest"
echo ""

# Step 2: Build old driver image
echo "→ Building old driver image..."
docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --target old \
  -t python-perf-driver-old:latest .

echo ""
echo "✓ Built: python-perf-driver-old:latest"
