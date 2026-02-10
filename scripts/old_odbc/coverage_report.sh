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

# Print nice formatted summary
echo ""
echo "Old ODBC Driver Coverage Summary"
echo "========================================"

# Parse and display line coverage
LINE_INFO=$(grep "lines" "$COVERAGE_DIR/summary.txt" | head -1)
if [ -n "$LINE_INFO" ]; then
    LINE_PCT=$(echo "$LINE_INFO" | sed -E 's/.*: ([0-9.]+)%.*/\1/')
    LINE_HIT=$(echo "$LINE_INFO" | sed -E 's/.*\(([0-9]+) of.*/\1/')
    LINE_TOTAL=$(echo "$LINE_INFO" | sed -E 's/.*of ([0-9]+).*/\1/')
    echo "  Line coverage:     ${LINE_PCT}% (${LINE_HIT}/${LINE_TOTAL} lines)"
fi

# Parse and display function coverage
FUNC_INFO=$(grep "functions" "$COVERAGE_DIR/summary.txt" | head -1)
if [ -n "$FUNC_INFO" ] && [[ "$FUNC_INFO" != *"no data"* ]]; then
    FUNC_PCT=$(echo "$FUNC_INFO" | sed -E 's/.*: ([0-9.]+)%.*/\1/')
    FUNC_HIT=$(echo "$FUNC_INFO" | sed -E 's/.*\(([0-9]+) of.*/\1/')
    FUNC_TOTAL=$(echo "$FUNC_INFO" | sed -E 's/.*of ([0-9]+).*/\1/')
    echo "  Function coverage: ${FUNC_PCT}% (${FUNC_HIT}/${FUNC_TOTAL} functions)"
fi

echo ""
echo "HTML report: $COVERAGE_DIR/html/index.html"


