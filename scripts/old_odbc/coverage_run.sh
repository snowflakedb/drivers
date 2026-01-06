#!/bin/bash

# Generate coverage report for old ODBC driver using new ODBC tests.
# This helps us estimate how good are our new ODBC tests and by extension how good is our new ODBC driver.

source /root/.awscredentials

# . ./scripts/decode_secrets.sh

cd old_odbc
ODBC_CODE_COVERAGE=1 ./Installer/gen_unix_installer.sh -r -t Release -p

export PARAMETER_PATH=$(pwd)/parameters.json
export DRIVER_PATH=$(pwd)/old_odbc/cmake_build/Source/libSnowflake.so
pushd odbc_tests
    rm -rf cmake-build
    mkdir -p cmake-build
    cmake4 -B cmake-build \
        -D DRIVER_TYPE=OLD \
        .

    export SIMBAINI=/usr/lib64/snowflake/odbc/lib/simba.snowflake.ini
    cmake4 --build cmake-build -- -j $(nproc)
    ctest4 -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure
popd


