#!/bin/bash

# Generate coverage report from gcov files.
# This script is meant to run inside the rocky-old-odbc-build container
# after tests have been executed.

set -e

echo "=== Generating coverage report ==="
GCOV_DIR=/workspace/old_odbc/cmake_build/Source/CMakeFiles/SnowflakeObject.dir
COVERAGE_DIR=/workspace/coverage_report

mkdir -p "$COVERAGE_DIR"

# Capture coverage data
lcov --capture \
    --directory "$GCOV_DIR" \
    --output-file "$COVERAGE_DIR/coverage.info" \
    --ignore-errors source \
    --include "**/Source/*" \
    --include "**/Dependencies/linux/Release/libsnowflakeclient/*"

lcov --summary "$COVERAGE_DIR/coverage.info" > "$COVERAGE_DIR/summary.txt"

# Generate HTML report
genhtml "$COVERAGE_DIR/coverage.info" \
    --output-directory "$COVERAGE_DIR/html" \
    --ignore-errors source

echo "Coverage report generated at: $COVERAGE_DIR/html/index.html"


