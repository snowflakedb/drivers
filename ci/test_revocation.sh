#!/bin/bash -e
#
# Test certificate revocation validation using the revocation-validation framework.
#

set -o pipefail

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DRIVER_ROOT="$( dirname "${THIS_DIR}")"
WORKSPACE=${WORKSPACE:-${DRIVER_ROOT}}

echo "[Info] Starting revocation validation tests"

# Ensure unzip is available (needed by protoc installer during cargo build)
if ! command -v unzip >/dev/null 2>&1; then
    echo "[Info] Installing unzip..."
    yum install -y unzip || apt-get install -y unzip || true
fi

# Clone revocation-validation framework
REVOCATION_BRANCH="${REVOCATION_BRANCH:-main}"
REVOCATION_REPO="https://github.com/snowflake-eng/revocation-validation.git"
REVOCATION_DIR="$(mktemp -d "${TMPDIR:-/tmp}/revocation-validation.XXXXXX")"
trap 'rm -rf "$REVOCATION_DIR"' EXIT
if [ -n "$GITHUB_USER" ] && [ -n "$GITHUB_TOKEN" ]; then
    git -c "http.${REVOCATION_REPO%.git}.extraheader=AUTHORIZATION: basic $(printf '%s:%s' "$GITHUB_USER" "$GITHUB_TOKEN" | base64)" \
        clone --depth 1 --branch "$REVOCATION_BRANCH" "$REVOCATION_REPO" "$REVOCATION_DIR"
else
    git clone --depth 1 --branch "$REVOCATION_BRANCH" "$REVOCATION_REPO" "$REVOCATION_DIR"
fi

cd "$REVOCATION_DIR"

echo "[Info] Running tests with Go $(go version | grep -oE 'go[0-9]+\.[0-9]+')..."

set +e
go run . \
    --client universal-driver-rust \
    --universal-driver-path "$DRIVER_ROOT" \
    --output "${WORKSPACE}/revocation-results.json" \
    --output-html "${WORKSPACE}/revocation-report.html" \
    --log-level debug
EXIT_CODE=$?
set -e

if [ -f "${WORKSPACE}/revocation-results.json" ] && [ -f "${WORKSPACE}/revocation-report.html" ]; then
    echo "[Info] Results: ${WORKSPACE}/revocation-results.json"
    echo "[Info] Report: ${WORKSPACE}/revocation-report.html"
else
    echo "[Warn] Expected output files were not generated"
fi

exit $EXIT_CODE
