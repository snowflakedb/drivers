#!/bin/bash
set -e

BUILDPLATFORM=${BUILDPLATFORM:-linux/amd64}

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building sf-core-builder"
echo "Platform: ${BUILDPLATFORM}"
echo ""

docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder:latest built successfully!"
