#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"

echo "Building WireMock container for performance testing..."

# Build Docker image with repo root as context so the local WireMock JAR is accessible
docker build -f "$SCRIPT_DIR/Dockerfile" -t wiremock-perf:latest "$REPO_ROOT"

echo "✓ WireMock container built successfully: wiremock-perf:latest"
