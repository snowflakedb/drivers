
#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Usage: $0 <example_name>"
    exit 1
fi

EXAMPLE_FILE=$1
SCRIPT_DIR="$(dirname "$0")"
pushd "$SCRIPT_DIR" > /dev/null

    if [ ! -f "$EXAMPLE_FILE.py" ]; then
        echo "Example file $EXAMPLE_FILE.py does not exist"
        exit 1
    fi
    export CORE_PATH="../../target/debug/libsf_core.dylib"
    source ../../pep249_dbapi/.venv/bin/activate
    python "$EXAMPLE_FILE.py"
popd > /dev/null

