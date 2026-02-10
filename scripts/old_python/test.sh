#!/bin/bash

# Run universal-driver Python tests against old snowflake-connector-python with coverage.
# 
# This measures how much of the OLD driver's source code is exercised by our
# comparable tests (integ + e2e). This helps us understand how comprehensive
# our test suite is.
#
# We run only tests/integ and tests/e2e - the same tests that run in
# python_tests_reference job, which are designed to work with both old and new drivers.

set -e

echo "=== Running comparable tests with coverage against old connector ==="

# Activate the virtual environment with old connector installed from source
source /workspace/python_coverage_venv/bin/activate

# Verify we're using the old connector
echo "Using connector version:"
python3 -c "import snowflake.connector; print(snowflake.connector.__version__)"

# Set test parameters
export PARAMETER_PATH=/workspace/parameters.json

# Create coverage report directory
mkdir -p /workspace/python_coverage_report

cd /workspace/python

# Auto-detect the old connector source path (structure varies by version)
# Try src/snowflake/connector first, then snowflake/connector
if [ -d "/workspace/old_python_connector/src/snowflake/connector" ]; then
    CONNECTOR_SOURCE=/workspace/old_python_connector/src/snowflake/connector
elif [ -d "/workspace/old_python_connector/snowflake/connector" ]; then
    CONNECTOR_SOURCE=/workspace/old_python_connector/snowflake/connector
else
    echo "Error: Could not find connector source in old_python_connector"
    echo "Checked: src/snowflake/connector and snowflake/connector"
    ls -la /workspace/old_python_connector/
    exit 1
fi

echo "Tracking coverage for OLD connector source at: ${CONNECTOR_SOURCE}"

# Run ONLY the comparable tests (integ + e2e) with coverage on the OLD connector
# These are the same tests that run in python_tests_reference
# Unit tests are skipped because they test UD-specific implementation
pytest tests/integ tests/e2e \
    --cov="${CONNECTOR_SOURCE}" \
    --cov-report=xml:/workspace/python_coverage_report/coverage.xml \
    --cov-report=html:/workspace/python_coverage_report/html \
    --cov-report=term \
    --json-report \
    --json-report-file=/workspace/python_coverage_report/test_results.json \
    -v \
    --tb=short \
    --continue-on-collection-errors \
    || true  # Continue even if some tests fail (expected due to behavioral differences)

echo "=== Comparable tests coverage completed ==="
echo "Coverage report: /workspace/python_coverage_report/html/index.html"

