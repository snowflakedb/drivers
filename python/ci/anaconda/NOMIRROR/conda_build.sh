#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECIPE_DIR="${SCRIPT_DIR}/../recipe"
PYTHON_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

PYTHON_VERSIONS="${PYTHON_VERSIONS:-3.10 3.11 3.12 3.13 3.14}"

if [[ -z "${SNOWFLAKE_CONNECTOR_PYTHON_VERSION:-}" ]]; then
  VERSION_FILE="${PYTHON_DIR}/src/snowflake/connector/version.py"
  if [[ ! -f "$VERSION_FILE" ]]; then
    echo "[FAILURE] version file not found: $VERSION_FILE" >&2
    exit 1
  fi
  SNOWFLAKE_CONNECTOR_PYTHON_VERSION="$(
    grep -Eo '__version__[[:space:]]*=[[:space:]]*"[^"]+"' "$VERSION_FILE" \
      | head -n1 \
      | sed -E 's/.*"([^"]+)".*/\1/'
  )"
  export SNOWFLAKE_CONNECTOR_PYTHON_VERSION
fi

echo "Building ${SNOWFLAKE_CONNECTOR_PYTHON_VERSION} for Python: ${PYTHON_VERSIONS}"

format_args=()
if [[ -n "${CONDA_PACKAGE_FORMAT:-}" ]]; then
  format_args=(--package-format "${CONDA_PACKAGE_FORMAT}")
fi

for py in ${PYTHON_VERSIONS}; do
  echo "===== conda build --python ${py} ====="
  conda build "${RECIPE_DIR}" --python "${py}" "${format_args[@]}"
done
