#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building WireMock container for performance testing..."

# Build Docker image
docker build -t wiremock-perf:latest .

echo "✓ WireMock container built successfully: wiremock-perf:latest"
