#!/bin/bash

# Build and run ODBC tests against the old driver.
# This script is meant to run inside the rocky-old-odbc-test container.

set -e

export PARAMETER_PATH=/workspace/parameters.json
export DRIVER_PATH=/workspace/old_odbc/cmake_build/Source/libSnowflake.so

cd /workspace/odbc_tests
rm -rf cmake-build
mkdir -p cmake-build
cmake4 -B cmake-build \
    -D DRIVER_TYPE=OLD \
    .

export SIMBAINI=/usr/lib64/snowflake/odbc/lib/simba.snowflake.ini
cmake4 --build cmake-build -- -j $(nproc)

# Don't fail the script on test failures — coverage should still be collected.
ctest4 -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure || true

