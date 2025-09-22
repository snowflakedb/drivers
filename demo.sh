#!/bin/bash

export PASSWORD=$(op read "op://Eng - Snow Drivers Warsaw/sfctest0 - aws - universal/password")
OLD_DRIVER="/opt/snowflake/snowflakeodbc/lib/universal/libSnowflake.dylib"
NEW_DRIVER="/Users/jszczerbinski/git/universal-driver-wt/NO-SNOW-dev-platform-demo/target/debug/libsfodbc.dylib"

CONNECTION_STRING="SERVER=sfctest0.snowflakecomputing.com;ACCOUNT=sfctest0;UID=test_universal;PWD=${PASSWORD};DATABASE=testdb_universal;SCHEMA=PUBLIC"

# Select 1
# echo "--- New driver ---"
# ./examples/odbc/run.sh select_1 "DRIVER=${NEW_DRIVER};${CONNECTION_STRING}"

# echo "--- Old driver ---"
# ./examples/odbc/run.sh select_1 "DRIVER=${OLD_DRIVER};${CONNECTION_STRING}"

# Put file
# echo "--- New driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${NEW_DRIVER};${CONNECTION_STRING}"

# echo "--- Old driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${OLD_DRIVER};${CONNECTION_STRING}"

# # Missing password
# MISSING_PASSWORD_CONNECTION_STRING="SERVER=sfctest0.snowflakecomputing.com;ACCOUNT=sfctest0;UID=test_universal;DATABASE=testdb_universal;SCHEMA=PUBLIC"
# echo "--- New driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${NEW_DRIVER};${MISSING_PASSWORD_CONNECTION_STRING}"

# echo "--- Old driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${OLD_DRIVER};${MISSING_PASSWORD_CONNECTION_STRING}"

# # Missing database
# MISSING_DB_CONNECTION_STRING="SERVER=sfctest0.snowflakecomputing.com;ACCOUNT=sfctest0;UID=test_universal;PWD=${PASSWORD};SCHEMA=PUBLIC"
# echo "--- New driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${NEW_DRIVER};${MISSING_DB_CONNECTION_STRING}"

# echo "--- Old driver ---"
# ./examples/odbc/run.sh put_file "DRIVER=${OLD_DRIVER};${MISSING_DB_CONNECTION_STRING}"

# # Python logging
# echo "--- Python ---"
# ./examples/python/run.sh select_1

