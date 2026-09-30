#!/bin/bash
#
# Build the local Node.js WIF artifact image (linux/amd64).
#
# Usage:
#   ./tests/docker/wif-nodejs/build.sh
#
# Optional environment variables:
#   PLATFORM   - docker build platform (default: linux/amd64)
#   IMAGE_TAG  - full image tag (default: ud-wif-nodejs-local:latest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PLATFORM="${PLATFORM:-linux/amd64}"
IMAGE_TAG="${IMAGE_TAG:-ud-wif-nodejs-local:latest}"

docker build \
  --platform="$PLATFORM" \
  --tag "$IMAGE_TAG" \
  "$SCRIPT_DIR"

echo "Built: ${IMAGE_TAG}"
