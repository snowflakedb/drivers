#!/bin/bash

set -e

# Generate Java code from thrift file
thrift -r --gen java --out jdbc/src/java/com/snowflake/jdbc/thrift_gen/ thrift/database_driver_v1.thrift
for SOURCE in jdbc/src/java/com/snowflake/jdbc/thrift_gen/*.java; do
    TEMP_FILE=$(mktemp)
    echo "package com.snowflake.jdbc.thrift_gen;" > ${TEMP_FILE}
    # Remove date parameter from @javax.annotation.Generated annotation
    sed 's/@javax\.annotation\.Generated(value = "\([^"]*\)", date = "[^"]*")/@javax.annotation.Generated(value = "\1")/g' ${SOURCE} >> ${TEMP_FILE}
    cat ${TEMP_FILE} > ${SOURCE}
    rm ${TEMP_FILE}
done

# Generate Rust code from thrift file
thrift -r --gen rs --out sf_core/src/thrift_gen/ thrift/database_driver_v1.thrift

# Generate Python code from thrift file
thrift -r --gen py --out pep249_dbapi/pep249_dbapi/thrift_gen/ thrift/database_driver_v1.thrift

