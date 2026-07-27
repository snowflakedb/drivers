#!/bin/bash
# Shared setup for requires_no_mfa auth tests (parameters_aws_local.json).

set -euo pipefail

WORKSPACE_ROOT="${WORKSPACE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
LOCAL_PARAMS="${WORKSPACE_ROOT}/.github/secrets/parameters_aws_local.json"

if [ ! -f "${LOCAL_PARAMS}" ]; then
    echo "=== Decoding secrets ==="
    (cd "${WORKSPACE_ROOT}" && ./scripts/decode_secrets.sh aws)
fi

if [ ! -f "${LOCAL_PARAMS}" ]; then
    echo "ERROR: ${LOCAL_PARAMS} not found" >&2
    echo "Run: ./scripts/decode_secrets.sh aws" >&2
    exit 1
fi

export PARAMETER_PATH="${LOCAL_PARAMS}"
export SF_TEST_NO_MFA=true
export CARGO_TARGET_DIR="${WORKSPACE_ROOT}/target"
