#!/bin/bash
#
# Build the external-browser test Docker image (linux/amd64).
#
# Usage:
#   ./tests/docker/external-browser/build.sh
#
# Optional environment variables:
#   PLATFORM   - docker build platform (default: linux/amd64)
#   IMAGE_TAG  - full image tag (default: snowflakedb/snowdrivers-test-external-browser-universal-driver:latest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PLATFORM="${PLATFORM:-linux/amd64}"
IMAGE_TAG="${IMAGE_TAG:-snowflakedb/snowdrivers-test-external-browser-universal-driver:latest}"

docker build \
    --platform="$PLATFORM" \
    --tag "$IMAGE_TAG" \
    "$SCRIPT_DIR"

echo "Built: ${IMAGE_TAG}"
echo ""
echo "To push to a remote registry:"
echo "  docker tag ${IMAGE_TAG} <registry>/<name>:<version>"
echo "  docker push <registry>/<name>:<version>"
