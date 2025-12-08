#!/bin/bash
# Run Snowflake's official ODBC test suite against the universal driver

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SNOWFLAKE_ODBC_REPO="/Users/snoonan/repos/snowflake-odbc"
PARAMS="$PROJECT_ROOT/parameters.json"
UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Snowflake ODBC Test Suite - Universal Driver${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Check prerequisites
if [ ! -d "$SNOWFLAKE_ODBC_REPO" ]; then
    echo -e "${RED}Error: Snowflake ODBC repo not found at $SNOWFLAKE_ODBC_REPO${NC}"
    exit 1
fi

if [ ! -f "$PARAMS" ]; then
    echo -e "${RED}Error: parameters.json not found at $PARAMS${NC}"
    exit 1
fi

if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo -e "${YELLOW}Building universal driver...${NC}"
    cd "$PROJECT_ROOT"
    cargo build -p odbc --release
fi

# Extract connection parameters from parameters.json
echo -e "${BLUE}Extracting connection parameters...${NC}"
ACCOUNT=$(grep "SNOWFLAKE_TEST_ACCOUNT" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
USER=$(grep "SNOWFLAKE_TEST_USER" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
PASSWORD=$(grep "SNOWFLAKE_TEST_PASSWORD\":" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
DATABASE=$(grep "SNOWFLAKE_TEST_DATABASE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
SCHEMA=$(grep "SNOWFLAKE_TEST_SCHEMA" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
WAREHOUSE=$(grep "SNOWFLAKE_TEST_WAREHOUSE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
ROLE=$(grep "SNOWFLAKE_TEST_ROLE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')

echo -e "${GREEN}✓ Connection parameters loaded${NC}"
echo "  Account: $ACCOUNT"
echo "  User: $USER"
echo "  Database: $DATABASE"
echo ""

# Create temporary ODBC configuration files
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

ODBCINST_INI="$TEMP_DIR/odbcinst.ini"
ODBC_INI="$TEMP_DIR/odbc.ini"

echo -e "${BLUE}Creating ODBC configuration...${NC}"

# Create odbcinst.ini with our driver
cat > "$ODBCINST_INI" << EOF
[SnowflakeDSIIDriver]
Description=Snowflake Universal ODBC Driver
Driver=$UNIVERSAL_DRIVER
EOF

# Create odbc.ini with DSN configuration
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

[SnowflakeDSIIAdmin]
Description=Snowflake Universal Driver Admin Test DSN
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
echo "  ODBCINST: $ODBCINST_INI"
echo "  ODBC: $ODBC_INI"
echo ""

# Set ODBC environment variables
export ODBCSYSINI="$TEMP_DIR"
export ODBCINI="$ODBC_INI"
export ODBCINSTINI="$ODBCINST_INI"

# Build a simple test from Snowflake's test suite
echo -e "${BLUE}Building Snowflake test suite...${NC}"

cd "$SNOWFLAKE_ODBC_REPO"

# Create a minimal CMakeLists.txt for just the tests we want to run
TEST_BUILD_DIR="$TEMP_DIR/test_build"
mkdir -p "$TEST_BUILD_DIR"

# Copy the test we want to run
cp -r Tests/EndToEndTests/ApiCatchLatestTest "$TEST_BUILD_DIR/"
cp Tests/ODBCClassCatch.hpp "$TEST_BUILD_DIR/"
cp Tests/FileUtil.hpp "$TEST_BUILD_DIR/"
cp Tests/EnvUtil.hpp "$TEST_BUILD_DIR/"

# Create a standalone CMakeLists.txt
cat > "$TEST_BUILD_DIR/CMakeLists.txt" << 'EOF'
cmake_minimum_required(VERSION 3.17)
project(SnowflakeUniversalDriverTests)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find unixODBC
find_package(ODBC REQUIRED)

# Add Catch2
Include(FetchContent)
FetchContent_Declare(
  Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG        v3.8.1
)
FetchContent_MakeAvailable(Catch2)

add_executable(ApiCatchLatestTest
    ApiCatchLatestTest/ApiCatchLatestTest.cpp
)

target_include_directories(ApiCatchLatestTest PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${ODBC_INCLUDE_DIRS}
)

target_link_libraries(ApiCatchLatestTest
    Catch2::Catch2WithMain
    ${ODBC_LIBRARIES}
)
EOF

cd "$TEST_BUILD_DIR"
mkdir -p build
cd build

echo -e "${YELLOW}Running CMake...${NC}"
cmake .. >/dev/null 2>&1 || {
    echo -e "${RED}✗ CMake configuration failed${NC}"
    exit 1
}

echo -e "${YELLOW}Building tests...${NC}"
make -j8 >/dev/null 2>&1 || {
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
}

echo -e "${GREEN}✓ Tests built successfully${NC}"
echo ""

# Run the tests
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Running Tests${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

RUST_LOG=${RUST_LOG:-warn} ./ApiCatchLatestTest 2>&1 | tee "$TEMP_DIR/test_output.txt"

# Parse results
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Test Results Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"

PASSED=$(grep -c "passed" "$TEMP_DIR/test_output.txt" || echo "0")
FAILED=$(grep -c "failed" "$TEMP_DIR/test_output.txt" || echo "0")

if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
else
    echo -e "${YELLOW}⚠ Some tests failed${NC}"
    echo -e "  Passed: ${GREEN}$PASSED${NC}"
    echo -e "  Failed: ${RED}$FAILED${NC}"
fi

echo ""
echo -e "${BLUE}Full test output saved to: $TEMP_DIR/test_output.txt${NC}"

