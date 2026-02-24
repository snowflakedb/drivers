#!/bin/bash

set -euo pipefail

MYPY_PROTOBUF_VERSION="~=5.0"
PROTOBUF_VERSION="~=6.33"
TYPES_PROTOBUF_VERSION="~=6.32"

if [[ -n "${PYTHON:-}" ]]; then
  PYTHON_BIN="${PYTHON}"
elif command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_BIN="python"
else
  echo "Error: no python interpreter found" >&2
  exit 1
fi

echo "--- Installing mypy-protobuf for Python .pyi stub generation ---"
"${PYTHON_BIN}" -m pip install \
  "mypy-protobuf${MYPY_PROTOBUF_VERSION}" \
  "protobuf${PROTOBUF_VERSION}" \
  "types-protobuf${TYPES_PROTOBUF_VERSION}"

# Ensure the pip user bin dir is visible so protoc can find protoc-gen-mypy.
USER_BASE="$("${PYTHON_BIN}" -m site --user-base 2>/dev/null || true)"
if [[ -n "${USER_BASE}" && -d "${USER_BASE}/bin" ]]; then
  export PATH="${USER_BASE}/bin:${PATH}"
fi

if ! command -v protoc-gen-mypy >/dev/null 2>&1; then
  echo "Error: protoc-gen-mypy not found on PATH" >&2
  exit 1
fi

echo "--- Generating proto (Rust is generated on the fly via build.rs) ---"
cargo run --bin proto_generator -- --generator python --input protobuf/database_driver_v1.proto --output python/src/snowflake/connector/_internal/protobuf_gen/
cargo run --bin proto_generator -- --generator java --input protobuf/database_driver_v1.proto --output jdbc/src/main/java/

