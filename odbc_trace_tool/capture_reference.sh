#!/bin/bash
# Capture obscured SQLGetData values from a replay test IR against the OLD
# (reference) driver, apply them into ir.yaml, and regenerate test.cpp.
#
# Usage: odbc_trace_tool/capture_reference.sh <replay_test_dir>
#
# Example:
#   odbc_trace_tool/capture_reference.sh \
#     odbc_tests/tests/replay/excel/powerquery/raw_sql/all_datatypes

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

TEST_DIR="${1:-}"
if [ -z "$TEST_DIR" ]; then
  echo "Usage: $0 <replay_test_dir>" >&2
  exit 1
fi

if [ ! -f "$TEST_DIR/ir.yaml" ]; then
  echo "Error: $TEST_DIR/ir.yaml not found" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Error: Docker is not running" >&2
  exit 1
fi

CAPTURE_CPP="$PROJECT_ROOT/odbc_tests/tests/capture_harness/capture.cpp"
CAPTURE_JSON="$PROJECT_ROOT/odbc_tests/tests/capture_harness/capture.json"
BUILD_DIR="$PROJECT_ROOT/odbc_tests/cmake-build-capture"
IR="$TEST_DIR/ir.yaml"

REFERENCE_ODBC_VERSION=$(cat "$PROJECT_ROOT/ci/reference-odbc-version" | tr -d '[:space:]')
. "$PROJECT_ROOT/ci/reference-odbc-checksums"

HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" = "x86_64" ]; then
  REFERENCE_ODBC_SHA256="$DEB_X86_64_SHA256"
else
  REFERENCE_ODBC_SHA256="$DEB_AARCH64_SHA256"
fi

cleanup() {
  rm -f "$CAPTURE_CPP" "$CAPTURE_JSON"
  rm -rf "$BUILD_DIR"
}
trap cleanup EXIT

# Pre-clean so a stale capture.json from an aborted run can never be applied.
rm -f "$CAPTURE_CPP" "$CAPTURE_JSON"
rm -rf "$BUILD_DIR"

echo "Generating capture harness from $IR ..."
cargo run -p odbc_trace_tool -- generate \
  --emit-capture-harness \
  -i "$IR" \
  -o "$CAPTURE_CPP"

echo "Building Docker image for ODBC reference tests ..."
docker build \
  --build-arg REFERENCE_ODBC_VERSION="$REFERENCE_ODBC_VERSION" \
  --build-arg REFERENCE_ODBC_SHA256="$REFERENCE_ODBC_SHA256" \
  -t odbc-reference-tests "$PROJECT_ROOT/odbc_tests"

echo "Running capture harness against OLD driver ..."
set +e
docker run --rm \
  -v "$PROJECT_ROOT":/workspace \
  -w /workspace \
  -e DRIVER_PATH="/usr/lib/snowflake/odbc/lib/libSnowflake.so" \
  -e PARAMETER_PATH="/workspace/parameters.json" \
  -e GIT_ROOT="/workspace" \
  -e CAPTURE_OUTPUT_PATH="/workspace/odbc_tests/tests/capture_harness/capture.json" \
  odbc-reference-tests \
  bash -c "
    set -e
    set -x
    if [ ! -f /workspace/parameters.json ]; then
      echo 'Error: parameters.json not found. Please run ./scripts/decode_secrets.sh first.'
      exit 1
    fi
    ODBC_LIB=\$(find /usr/lib -name 'libodbc.so' -print -quit)
    cd /workspace/odbc_tests/
    mkdir -p cmake-build-capture
    cmake -B cmake-build-capture \\
      -D ODBC_LIBRARY=\"\$ODBC_LIB\" \\
      -D ODBC_INCLUDE_DIR='/usr/include' \\
      -D DRIVER_TYPE=OLD \\
      -D BUILD_CAPTURE_HARNESS=ON \\
      -D CMAKE_CXX_COMPILER_LAUNCHER=ccache \\
      -D CMAKE_C_COMPILER_LAUNCHER=ccache \\
      .
    cmake --build cmake-build-capture --target capture_harness -- -j \$(nproc)
    ctest -C Debug --test-dir cmake-build-capture -R capture_harness --output-on-failure
  "
CTEST_EXIT=$?
set -e

if [ "$CTEST_EXIT" -ne 0 ]; then
  echo "Error: capture harness failed (ctest exit $CTEST_EXIT); ir.yaml left untouched" >&2
  exit "$CTEST_EXIT"
fi

if [ ! -f "$CAPTURE_JSON" ]; then
  echo "Error: capture.json was not written" >&2
  exit 1
fi

echo "Applying captured values to $IR ..."
cargo run -p odbc_trace_tool -- apply-capture \
  -i "$IR" \
  --values "$CAPTURE_JSON"

echo "Regenerating test.cpp ..."
TEST_BASENAME="$(basename "$TEST_DIR")"
cargo run -p odbc_trace_tool -- generate \
  -i "$IR" \
  -o "$TEST_DIR/test.cpp" \
  -n "excel powerquery raw_sql ${TEST_BASENAME}" \
  -t "excel][powerquery][raw_sql"

echo "Capture complete: $IR and $TEST_DIR/test.cpp updated"
