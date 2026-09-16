#!/bin/bash
set -e

SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building ADBC performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/adbc/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t adbc-perf-driver:latest .

echo ""
echo "✓ Built: adbc-perf-driver:latest"
