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
#   DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh <python|odbc|jdbc> [mode]
#
# The optional mode selects which driver to test (default: universal):
#   universal   The universal driver (new). Default.
#   reference   The legacy driver, to verify old-driver compatibility.
#
# Examples:
#   DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh jdbc            # universal
#   DOCKER_IMAGE=<image> ./tests/auth/run_auth_browser.sh jdbc reference  # legacy
#
# Optional: DOCKER_RUN_EXTRA_ARGS — extra flags appended to docker run (e.g. "-e VAR"
# from a CI caller that needs to forward env into the container).
#
# Local development (builds the image automatically):
#   ./tests/auth/run_auth_browser_local.sh python

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

WRAPPER="${1:-}"
if [ -z "$WRAPPER" ] || [ ! -f "${SCRIPT_DIR}/auth_browser_${WRAPPER}.sh" ]; then
    echo "Usage: $0 <python|odbc|jdbc> [universal|reference]" >&2
    exit 1
fi

AUTH_BROWSER_MODE="${2:-universal}"
if [ "$AUTH_BROWSER_MODE" != "universal" ] && [ "$AUTH_BROWSER_MODE" != "reference" ]; then
    echo "ERROR: unknown mode '$AUTH_BROWSER_MODE' (expected 'universal' or 'reference')" >&2
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

HELPERS="${REPO_ROOT}/tests/docker/external-browser/browser-helpers"
# Image :4 still ships generateTotp() as { current, past, future } and
# `const { current } = await totp.generateTotp()`. Overlay both the generator
# (bare current-window string) and the Playwright helper that fills it, or
# page.fill gets undefined and OAuth MFA times out.
docker run --rm --platform "$PLATFORM" \
    -v "${REPO_ROOT}:/mnt/host" \
    -v "${HELPERS}/totpGenerator.js:/externalbrowser/totpGenerator.js:ro" \
    -v "${HELPERS}/totpGenerator.test.js:/externalbrowser/totpGenerator.test.js:ro" \
    -v "${HELPERS}/provideBrowserCredentials.js:/externalbrowser/provideBrowserCredentials.js:ro" \
    -v "${HELPERS}/getTOTP.js:/externalbrowser/getTOTP.js:ro" \
    -e WORKSPACE_ROOT=/mnt/host \
    -e WORKSPACE=/mnt/host \
    -e BUILD_TAG="${BUILD_TAG:-local}" \
    -e AUTH_BROWSER_MODE="${AUTH_BROWSER_MODE}" \
    ${DOCKER_RUN_EXTRA_ARGS:-} \
    "${DOCKER_IMAGE}" \
    bash -c "cd /externalbrowser && node --test totpGenerator.test.js && bash /mnt/host/tests/auth/auth_browser_${WRAPPER}.sh"
