#!/bin/bash
set -e

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

source tests/performance/drivers/detect_platform.sh

echo "Building sf-core-builder"
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder:latest built successfully!"
