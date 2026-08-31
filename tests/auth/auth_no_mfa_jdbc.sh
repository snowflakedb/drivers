#!/bin/bash
# JDBC requires_no_mfa tests (parameters_aws_local.json).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_no_mfa_common.sh"

cd "${WORKSPACE_ROOT}"
if [[ "${JDBC_BRIDGE_CACHE_HIT:-false}" == "true" ]]; then
  echo "=== Using cached JDBC bridge (libjdbc_bridge) ==="
else
  echo "=== Building JDBC bridge (libjdbc_bridge) ==="
  cargo build --locked -p jdbc_bridge
fi

echo ""
echo "=== Running JDBC requires_no_mfa tests ==="
cd "${WORKSPACE_ROOT}/jdbc"
GRADLE_INCLUDE_TAGS=requires_no_mfa GRADLE_MAX_PARALLEL_FORKS=1 ./gradlew test
