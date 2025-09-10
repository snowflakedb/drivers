#!/bin/bash

ODBC_CFLAGS=$(odbc_config --cflags)
ODBC_LIBS=$(odbc_config --libs)

SCRIPT_DIR="$(dirname "$0")"
FILES="main.c select_1.c put_file.c get_file.c"

pushd "$SCRIPT_DIR" > /dev/null
    gcc -g -Wall -Wextra -pedantic ${ODBC_CFLAGS} -o example ${FILES} ${ODBC_LIBS} 2> compile.log
    if [ $? -ne 0 ]; then
        cat compile.log
        exit 1
    else
        warnings=$(grep -c "warning:" compile.log)
        if [ $warnings -ne 0 ]; then
            echo "--- Compilation generated $warnings warnings. Please check compile.log for details. ---"
        fi
        echo "--- Compilation succeeded ---"
    fi
    echo "--- Running example ---"
    ./example $@
popd > /dev/null
