#!/bin/bash
#
# Build the Docker image locally and run auth E2E tests.
# Convenience wrapper for local development — no Artifactory access needed.
#
# Usage:
#   ./tests/auth/run_auth_browser_local.sh python
#   ./tests/auth/run_auth_browser_local.sh odbc
#   ./tests/auth/run_auth_browser_local.sh jdbc

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_SH="${REPO_ROOT}/tests/docker/external-browser/build.sh"
LOCAL_IMAGE="ud-external-browser:local"

ARCH="$(uname -m)"
if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
    PLATFORM="linux/arm64"
else
    PLATFORM="linux/amd64"
fi

echo "=== Building image for ${PLATFORM} ==="
PLATFORM="$PLATFORM" IMAGE_TAG="$LOCAL_IMAGE" "$BUILD_SH"

echo "=== Running auth browser tests (${1:-}) ==="
export DOCKER_IMAGE="$LOCAL_IMAGE"
exec "${SCRIPT_DIR}/run_auth_browser.sh" "$@"
