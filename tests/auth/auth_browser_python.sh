#!/bin/bash
#
# Python authentication E2E tests requiring a headless browser.
# Runs inside the snowdrivers-test-external-browser-universal-driver Docker image.
#
# Local usage (requires parameters_preprod.json in repo root,
# from ./scripts/decode_secrets.sh preprod parameters_preprod.json):
#
#   docker run --rm --platform linux/amd64 \
#     -v "$PWD:/mnt/host" -e WORKSPACE_ROOT=/mnt/host \
#     snowdrivers-test-external-browser-universal-driver:1 \
#     bash /mnt/host/tests/auth/auth_browser_python.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

echo "=== Creating virtualenv ==="
# Use an isolated venv rather than --break-system-packages: the base image's
# Python is externally managed (PEP 668) and we must not mutate it.
VENV_DIR="${WORKSPACE_ROOT}/.venv-auth-browser"
python3 -m venv "${VENV_DIR}"
# shellcheck disable=SC1091
source "${VENV_DIR}/bin/activate"

echo "=== Installing Python connector (building sf_core from source) ==="
cd "${WORKSPACE_ROOT}/python"
pip install -e ".[dev,test]"

echo ""
echo "=== Running Python auth browser E2E tests ==="
python3 -m pytest tests/e2e/authentication/ \
    -v -m requires_browser -n 0
