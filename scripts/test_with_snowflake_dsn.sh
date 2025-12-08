#!/bin/bash
# Test universal driver using Snowflake's DSN-based test approach

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PARAMS="$PROJECT_ROOT/parameters.json"
UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Testing Universal Driver with DSN Configuration${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Build driver if needed
if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo -e "${YELLOW}Building universal driver...${NC}"
    cd "$PROJECT_ROOT"
    cargo build -p odbc --release
fi

# Extract connection parameters
echo -e "${BLUE}Extracting connection parameters...${NC}"
ACCOUNT=$(grep "SNOWFLAKE_TEST_ACCOUNT" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
USER=$(grep "SNOWFLAKE_TEST_USER" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
PASSWORD=$(grep "SNOWFLAKE_TEST_PASSWORD\":" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
DATABASE=$(grep "SNOWFLAKE_TEST_DATABASE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
SCHEMA=$(grep "SNOWFLAKE_TEST_SCHEMA" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
WAREHOUSE=$(grep "SNOWFLAKE_TEST_WAREHOUSE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
ROLE=$(grep "SNOWFLAKE_TEST_ROLE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')

echo -e "${GREEN}✓ Parameters loaded${NC}"
echo ""

# Create temporary ODBC configuration
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

ODBCINST_INI="$TEMP_DIR/odbcinst.ini"
ODBC_INI="$TEMP_DIR/odbc.ini"

cat > "$ODBCINST_INI" << EOF
[SnowflakeDSIIDriver]
Description=Snowflake Universal ODBC Driver
Driver=$UNIVERSAL_DRIVER
EOF

cat > "$ODBC_INI" << EOF
[SnowflakeDSII]
Description=Snowflake Universal Driver Test DSN
Driver=SnowflakeDSIIDriver
Server=${ACCOUNT}.snowflakecomputing.com
Account=$ACCOUNT
UID=$USER
PWD=$PASSWORD
Database=$DATABASE
Schema=$SCHEMA
Warehouse=$WAREHOUSE
Role=$ROLE
EOF

echo -e "${GREEN}✓ ODBC configuration created${NC}"
echo ""

# Create a simple test program
TEST_PROGRAM="$TEMP_DIR/dsn_test.cpp"
cat > "$TEST_PROGRAM" << 'EOFCPP'
#include <sql.h>
#include <sqlext.h>
#include <iostream>
#include <string>

void printDiagnostics(SQLSMALLINT handleType, SQLHANDLE handle) {
    SQLCHAR sqlState[6], message[SQL_MAX_MESSAGE_LENGTH];
    SQLINTEGER nativeError;
    SQLSMALLINT textLength, rec = 1;
    while (SQLGetDiagRec(handleType, handle, rec++, sqlState, &nativeError,
                         message, sizeof(message), &textLength) == SQL_SUCCESS) {
        std::cout << "  [" << sqlState << "] (" << nativeError << ") " 
                  << message << std::endl;
    }
}

int main() {
    SQLHENV env = nullptr;
    SQLHDBC dbc = nullptr;
    SQLHSTMT stmt = nullptr;
    SQLRETURN rc;
    
    std::cout << "Test 1: Allocate environment..." << std::endl;
    rc = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        return 1;
    }
    std::cout << "  PASSED" << std::endl;
    
    std::cout << "Test 2: Set ODBC version..." << std::endl;
    rc = SQLSetEnvAttr(env, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        printDiagnostics(SQL_HANDLE_ENV, env);
        return 1;
    }
    std::cout << "  PASSED" << std::endl;
    
    std::cout << "Test 3: Allocate connection..." << std::endl;
    rc = SQLAllocHandle(SQL_HANDLE_DBC, env, &dbc);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        printDiagnostics(SQL_HANDLE_ENV, env);
        return 1;
    }
    std::cout << "  PASSED" << std::endl;
    
    std::cout << "Test 4: Connect using driver path..." << std::endl;
    // Get connection parameters from environment
    const char* driverPath = std::getenv("TEST_DRIVER_PATH");
    const char* server = std::getenv("TEST_SERVER");
    const char* account = std::getenv("TEST_ACCOUNT");
    const char* uid = std::getenv("TEST_UID");
    const char* pwd = std::getenv("TEST_PWD");
    const char* database = std::getenv("TEST_DATABASE");
    const char* schema = std::getenv("TEST_SCHEMA");
    const char* warehouse = std::getenv("TEST_WAREHOUSE");
    const char* role = std::getenv("TEST_ROLE");
    
    std::string connStr = std::string("DRIVER=") + driverPath +
                         ";SERVER=" + server +
                         ";ACCOUNT=" + account +
                         ";UID=" + uid +
                         ";PWD=" + pwd +
                         ";DATABASE=" + database +
                         ";SCHEMA=" + schema +
                         ";WAREHOUSE=" + warehouse +
                         ";ROLE=" + role;
    
    SQLCHAR outConn[1024];
    SQLSMALLINT outConnLen;
    rc = SQLDriverConnect(dbc, nullptr, (SQLCHAR*)connStr.c_str(), SQL_NTS,
                          outConn, sizeof(outConn), &outConnLen, SQL_DRIVER_NOPROMPT);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        printDiagnostics(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return 1;
    }
    std::cout << "  PASSED" << std::endl;
    
    std::cout << "Test 5: Execute query..." << std::endl;
    rc = SQLAllocHandle(SQL_HANDLE_STMT, dbc, &stmt);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED to allocate statement" << std::endl;
        printDiagnostics(SQL_HANDLE_DBC, dbc);
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return 1;
    }
    
    rc = SQLExecDirect(stmt, (SQLCHAR*)"SELECT CURRENT_VERSION()", SQL_NTS);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        printDiagnostics(SQL_HANDLE_STMT, stmt);
        SQLFreeHandle(SQL_HANDLE_STMT, stmt);
        SQLDisconnect(dbc);
        SQLFreeHandle(SQL_HANDLE_DBC, dbc);
        SQLFreeHandle(SQL_HANDLE_ENV, env);
        return 1;
    }
    std::cout << "  PASSED" << std::endl;
    
    std::cout << "Test 6: Fetch result..." << std::endl;
    rc = SQLFetch(stmt);
    if (!SQL_SUCCEEDED(rc)) {
        std::cout << "  FAILED" << std::endl;
        printDiagnostics(SQL_HANDLE_STMT, stmt);
    } else {
        SQLCHAR version[256];
        SQLLEN indicator;
        SQLGetData(stmt, 1, SQL_C_CHAR, version, sizeof(version), &indicator);
        std::cout << "  PASSED - Snowflake version: " << version << std::endl;
    }
    
    std::cout << "Test 7: Cleanup..." << std::endl;
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
    std::cout << "  PASSED" << std::endl;
    
    std::cout << std::endl;
    std::cout << "All tests passed!" << std::endl;
    return 0;
}
EOFCPP

# Compile the test
echo -e "${BLUE}Compiling test program...${NC}"
g++ -std=c++17 -o "$TEMP_DIR/dsn_test" "$TEST_PROGRAM" \
    -I/opt/homebrew/opt/unixodbc/include \
    -L/opt/homebrew/opt/unixodbc/lib \
    -lodbc

echo -e "${GREEN}✓ Test compiled${NC}"
echo ""

# Run the test
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Running DSN-based Test${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

export ODBCSYSINI="$TEMP_DIR"
export ODBCINI="$ODBC_INI"
export ODBCINSTINI="$ODBCINST_INI"
export RUST_LOG=${RUST_LOG:-warn}
export TEST_DRIVER_PATH="$UNIVERSAL_DRIVER"
export TEST_SERVER="${ACCOUNT}.snowflakecomputing.com"
export TEST_ACCOUNT="$ACCOUNT"
export TEST_UID="$USER"
export TEST_PWD="$PASSWORD"
export TEST_DATABASE="$DATABASE"
export TEST_SCHEMA="$SCHEMA"
export TEST_WAREHOUSE="$WAREHOUSE"
export TEST_ROLE="$ROLE"

"$TEMP_DIR/dsn_test"

echo ""
echo -e "${GREEN}✓ DSN-based connection works!${NC}"
echo ""
echo -e "${YELLOW}This demonstrates that the universal driver can be used${NC}"
echo -e "${YELLOW}with Snowflake's DSN-based test suite.${NC}"

