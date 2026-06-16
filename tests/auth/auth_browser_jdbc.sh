#!/bin/bash
#
# JDBC authentication E2E tests requiring a headless browser container.
# Runs the external-browser (headless Chromium) suite inside
# snowdrivers-test-external-browser-universal-driver.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_browser_common.sh"

echo "=== Building JDBC bridge (libjdbc_bridge) ==="
cd "${WORKSPACE_ROOT}"
cargo build -p jdbc_bridge

echo ""
echo "=== Running JDBC auth browser E2E tests (tag: requires_browser) ==="
cd "${WORKSPACE_ROOT}/jdbc"
GRADLE_INCLUDE_TAGS=requires_browser ./gradlew test
