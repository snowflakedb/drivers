#!/bin/bash
# Python requires_no_mfa tests (parameters_aws_local.json).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/auth_no_mfa_common.sh"

echo "=== Creating virtualenv ==="
VENV_DIR="${WORKSPACE_ROOT}/.venv-auth-no-mfa"
python3 -m venv "${VENV_DIR}"
# shellcheck disable=SC1091
source "${VENV_DIR}/bin/activate"

cd "${WORKSPACE_ROOT}/python"

echo "=== Installing Python connector (building sf_core from source) ==="
pip install -e ".[dev,test]"

echo ""
echo "=== Running Python requires_no_mfa tests ==="
python3 -m pytest tests/e2e/authentication/test_user_password.py \
    -v -m requires_no_mfa -n 0
