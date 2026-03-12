#!/bin/bash

set -e
set -x
# Build and run ODBC tests using CMake
# Requires odbc_config to be available in PATH

cargo build

if [[ "$OSTYPE" == "darwin"* ]]; then
    DYLIB_EXT="dylib"
else
    DYLIB_EXT="so"
fi

export DRIVER_PATH=$(pwd)/target/debug/libsfodbc.${DYLIB_EXT}
export PARAMETER_PATH=${PARAMETER_PATH:-$(pwd)/parameters.json}

pushd odbc_tests
    if [ ! -d cmake-build ]; then
        mkdir -p cmake-build
        cmake -B cmake-build \
            -DCMAKE_CXX_FLAGS="-O0" \
            -DCMAKE_BUILD_TYPE=Debug \
            -D ODBC_LIBRARY="$(odbc_config --lib-prefix)/libodbc.${DYLIB_EXT}" \
            -D ODBC_INCLUDE_DIR="$(odbc_config --include-prefix)" \
            -D DRIVER_TYPE=NEW \
            .
    fi
    cmake --build cmake-build -- -j 16
    ctest -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure "$@"
popd
