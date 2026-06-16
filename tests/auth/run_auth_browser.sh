#!/bin/bash
#
# Spin up the headless browser Docker container and run auth E2E tests.
# Used by Jenkinsfile.auth-browser-tests and locally.
#
# Prerequisites:
#   - Docker running
#   - parameters_preprod.json in repo root (from ./scripts/decode_secrets.sh preprod parameters_preprod.json)
#
# Usage:
#   DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh python
#   DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh odbc
#
# Local development (builds the image automatically):
#   ./tests/auth/run_auth_browser_local.sh python

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

WRAPPER="${1:-}"
if [ -z "$WRAPPER" ] || [ ! -f "${SCRIPT_DIR}/auth_browser_${WRAPPER}.sh" ]; then
    echo "Usage: $0 <python|odbc|jdbc>" >&2
    exit 1
fi

if [ -z "${DOCKER_IMAGE:-}" ]; then
    echo "ERROR: DOCKER_IMAGE must be set" >&2
    echo "Example: DOCKER_IMAGE=my-image:tag $0 python" >&2
    exit 1
fi

ARCH="$(uname -m)"
if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
    PLATFORM="linux/arm64"
else
    PLATFORM="linux/amd64"
fi

docker run --rm --platform "$PLATFORM" \
    -v "${REPO_ROOT}:/mnt/host" \
    -e WORKSPACE_ROOT=/mnt/host \
    "${DOCKER_IMAGE}" \
    bash "/mnt/host/tests/auth/auth_browser_${WRAPPER}.sh"
