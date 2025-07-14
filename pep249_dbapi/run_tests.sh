
cargo build --package sf_core

if [ -z "$CORE_PATH" ]; then
    if [[ "$(uname)" == "Darwin" ]]; then
        export CORE_PATH=$(pwd)/target/debug/libsf_core.dylib
    else
        export CORE_PATH=$(pwd)/target/debug/libsf_core.so
    fi
fi

if [ -z "$PARAMETER_PATH" ]; then
    export PARAMETER_PATH=$(pwd)/parameters.json
fi

cd pep249_dbapi

pip install -e ".[dev]"

pytest tests/ -v
