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

CCACHE_ARGS=""
if command -v ccache &>/dev/null; then
    CCACHE_ARGS="-DCMAKE_CXX_COMPILER_LAUNCHER=ccache -DCMAKE_C_COMPILER_LAUNCHER=ccache"
fi

pushd odbc_tests
    if [ ! -d cmake-build ]; then
        mkdir -p cmake-build
        cmake -B cmake-build \
            -DCMAKE_CXX_FLAGS="-O0" \
            -DCMAKE_BUILD_TYPE=Debug \
            -D ODBC_LIBRARY="$(odbc_config --lib-prefix)/libodbc.${DYLIB_EXT}" \
            -D ODBC_INCLUDE_DIR="$(odbc_config --include-prefix)" \
            -D DRIVER_TYPE=NEW \
            ${CCACHE_ARGS} \
            .
    fi
    cmake --build cmake-build -- -j 16

    # --- Schema lifecycle ---
    SCHEMA_TOOL="$(pwd)/cmake-build/tools/schema_tool"
    if SCHEMA_NAME=$("$SCHEMA_TOOL" create); then
        if [[ ! "$SCHEMA_NAME" =~ ^TEMP_TEST_SCHEMA_[0-9]+$ ]]; then
            echo "run: schema_tool returned invalid name '$SCHEMA_NAME', falling back to per-process"
        else
            export ODBC_TEST_SCHEMA="$SCHEMA_NAME"
            trap '"$SCHEMA_TOOL" drop "$SCHEMA_NAME" 2>/dev/null || true' EXIT
            echo "run: using shared schema $SCHEMA_NAME"
        fi
    else
        echo "run: schema pre-creation failed, falling back to per-process"
    fi

    ctest -j $(nproc) -C Debug --test-dir cmake-build --output-on-failure "$@"
popd
