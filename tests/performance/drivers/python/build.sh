#!/bin/bash
set -e

# Auto-detect architecture if BUILDPLATFORM not set
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building Python performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/python/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t python-perf-driver:latest .

echo ""
echo "✓ Build complete: python-perf-driver:latest"
