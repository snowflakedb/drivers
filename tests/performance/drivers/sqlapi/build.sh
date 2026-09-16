#!/bin/bash
set -e

SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building SQL API performance driver..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/sqlapi/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sqlapi-perf-driver:latest .

echo ""
echo "✓ Built: sqlapi-perf-driver:latest"
