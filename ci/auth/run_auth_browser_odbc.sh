#!/bin/bash
#
# Spin up the headless browser Docker container and run ODBC auth E2E tests.
# Used by Jenkinsfile.auth-browser-tests and locally.
#
# Prerequisites:
#   - Docker running
#   - parameters.json in repo root
#
# Usage:
#   ./ci/auth/run_auth_browser_odbc.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKER_IMAGE="artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/internal-development-docker-drivers-local/snowflakedb/snowdrivers-test-external-browser-universal-driver:1"

docker run --rm --platform linux/amd64 \
    -v "${REPO_ROOT}:/mnt/host" \
    -e WORKSPACE_ROOT=/mnt/host \
    "${DOCKER_IMAGE}" \
    bash /mnt/host/ci/auth/auth_browser_odbc.sh
