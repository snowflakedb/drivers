#!/bin/bash -e
#
# Test certificate revocation validation using the revocation-validation framework.
#

set -o pipefail

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DRIVER_ROOT="$( dirname "${THIS_DIR}")"
WORKSPACE=${WORKSPACE:-${DRIVER_ROOT}}

echo "[Info] Starting revocation validation tests"

# Clone revocation-validation framework
REVOCATION_DIR="/tmp/revocation-validation"
REVOCATION_BRANCH="${REVOCATION_BRANCH:-main}"

rm -rf "$REVOCATION_DIR"
if [ -n "$GITHUB_USER" ] && [ -n "$GITHUB_TOKEN" ]; then
    git clone --depth 1 --branch "$REVOCATION_BRANCH" \
        "https://${GITHUB_USER}:${GITHUB_TOKEN}@github.com/snowflake-eng/revocation-validation.git" \
        "$REVOCATION_DIR"
else
    git clone --depth 1 --branch "$REVOCATION_BRANCH" \
        "https://github.com/snowflake-eng/revocation-validation.git" \
        "$REVOCATION_DIR"
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
