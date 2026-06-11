#!/bin/bash
#
# Build the external-browser test Docker image (linux/amd64).
#
# Usage:
#   ./tests/docker/external-browser/build.sh
#
# Optional environment variables:
#   PLATFORM   - docker build platform (default: linux/amd64)
#   IMAGE_TAG  - full local tag override (default: <image-name>:2)
#
# Tags the image locally. To push to a remote registry, retag and push:
#   docker tag <local-tag> <registry>/<local-tag>
#   docker push <registry>/<local-tag>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PLATFORM="${PLATFORM:-linux/amd64}"
IMAGE_NAME="snowflakedb/snowdrivers-test-external-browser-universal-driver"
IMAGE_VERSION="2"
LOCAL_TAG="${IMAGE_TAG:-${IMAGE_NAME}:${IMAGE_VERSION}}"

docker build \
    --platform="$PLATFORM" \
    --tag "$LOCAL_TAG" \
    "$SCRIPT_DIR"

echo "Built: ${LOCAL_TAG}"
echo ""
echo "To push to a remote registry:"
echo "  docker tag ${LOCAL_TAG} <registry>/${IMAGE_NAME}:${IMAGE_VERSION}"
echo "  docker push <registry>/${IMAGE_NAME}:${IMAGE_VERSION}"
