#!/bin/bash
set -e

# Auto-detect architecture if BUILDPLATFORM not set
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "${SCRIPT_DIR}/../detect_platform.sh"

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

echo "Building JDBC performance drivers..."
echo "Platform: ${BUILDPLATFORM}"
echo ""

# Step 1: Build sf-core-builder (shared with ODBC/Python; also builds libjdbc_bridge.so). The
# jdbc jar stage extends this image, so no host-side library extraction is needed.
echo "→ Building sf-core-builder (includes sf_core + jdbc_bridge)..."
docker build -f tests/performance/drivers/Dockerfile.sf_core_builder \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  -t sf-core-builder:latest .

echo ""
echo "✓ sf-core-builder ready"
echo ""

# Step 2: Build universal driver image (assembles the fat jar + compiles the perf app)
echo "→ Building universal driver image..."
docker build -f tests/performance/drivers/jdbc/Dockerfile \
  --build-arg BUILDPLATFORM="${BUILDPLATFORM}" \
  --build-arg JENKINS_HOME="${JENKINS_HOME:-}" \
  --target universal \
  -t jdbc-perf-driver-universal:latest .

echo ""
echo "✓ Built: jdbc-perf-driver-universal:latest"
