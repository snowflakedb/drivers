#!/bin/bash

# Generate coverage report for old ODBC driver using new ODBC tests.
# This helps us estimate how good are our new ODBC tests and by extension how good is our new ODBC driver.
#
# This script builds both docker images, then runs:
# 1. The build script in the build container (compiles old ODBC driver with coverage)
# 2. The test script in the test container (builds and runs tests against the driver)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Clone old ODBC if path doesn't exist
if [ -z "$OLD_ODBC_PATH" ] || [ ! -d "$OLD_ODBC_PATH" ]; then
    echo "=== Cloning old ODBC driver ==="
    OLD_ODBC_PATH="$PROJECT_DIR/old_odbc_cloned"
    if [ -d "$OLD_ODBC_PATH" ]; then
        rm -rf "$OLD_ODBC_PATH"
    fi
    git clone https://github.com/snowflakedb/snowflake-odbc-driver.git "$OLD_ODBC_PATH"
fi
AWS_CREDENTIALS="${AWS_CREDENTIALS:-$HOME/.awscredentials}"

# Source AWS credentials for passing as env vars to docker
source "$AWS_CREDENTIALS"

echo "=== Building Docker images ==="
docker build -f "$PROJECT_DIR/ci/Dockerfile.rocky-old-odbc-build" -t rocky-old-odbc-build "$PROJECT_DIR"
docker build -f "$PROJECT_DIR/ci/Dockerfile.rocky-old-odbc-test" -t rocky-old-odbc-test "$PROJECT_DIR"

echo "=== Step 1: Building old ODBC driver with coverage ==="
docker run --rm \
    -v "$PROJECT_DIR":/workspace \
    -v "$OLD_ODBC_PATH":/workspace/old_odbc \
    -e AWS_ACCESS_KEY_ID="$AWS_ACCESS_KEY_ID" \
    -e AWS_SECRET_ACCESS_KEY="$AWS_SECRET_ACCESS_KEY" \
    -w /workspace \
    rocky-old-odbc-build \
    ./scripts/old_odbc/build.sh

echo "=== Step 2: Building and running tests ==="
docker run --rm \
    -v "$PROJECT_DIR":/workspace \
    -v "$OLD_ODBC_PATH":/workspace/old_odbc \
    -e AWS_ACCESS_KEY_ID="$AWS_ACCESS_KEY_ID" \
    -e AWS_SECRET_ACCESS_KEY="$AWS_SECRET_ACCESS_KEY" \
    -w /workspace \
    rocky-old-odbc-test \
    ./scripts/old_odbc/test.sh

echo "=== Step 3: Generating coverage report ==="
docker run --rm \
    -v "$PROJECT_DIR":/workspace \
    -v "$OLD_ODBC_PATH":/workspace/old_odbc \
    -e AWS_ACCESS_KEY_ID="$AWS_ACCESS_KEY_ID" \
    -e AWS_SECRET_ACCESS_KEY="$AWS_SECRET_ACCESS_KEY" \
    -w /workspace \
    rocky-old-odbc-build \
    ./scripts/old_odbc/coverage_report.sh

echo "=== Coverage complete ==="


