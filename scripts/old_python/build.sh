#!/bin/bash

# Build old Python connector from source with coverage support.
# This script installs the old snowflake-connector-python from source
# so we can track coverage of the OLD driver's code when running our tests.
#
# This is meant to run inside the coverage Docker container.

set -e

echo "=== Setting up old Python connector for coverage ==="

# Create virtual environment for isolated testing
python3 -m venv /workspace/python_coverage_venv
source /workspace/python_coverage_venv/bin/activate

# Upgrade pip and install build dependencies
pip install --upgrade pip setuptools wheel

# Install build dependencies for snowflake-connector-python
pip install Cython "numpy<2" pyarrow

# Install the OLD connector from source in editable mode
# This ensures source files are available for coverage tracking
cd /workspace/old_python_connector

# Try to install with various optional dependencies
# Some versions may not have all extras available
pip install -e ".[development,pandas]" || \
pip install -e ".[pandas]" || \
pip install -e "."

# Install test dependencies
pip install pytest pytest-cov pytest-json-report pytest-timeout mock

echo ""
echo "=== Old Python connector installed successfully ==="
echo "Connector version:"
python3 -c "import snowflake.connector; print(snowflake.connector.__version__)"
echo "Connector location:"
python3 -c "import snowflake.connector; print(snowflake.connector.__file__)"

