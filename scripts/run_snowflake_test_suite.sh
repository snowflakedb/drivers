#!/bin/bash
# Build and run Snowflake's ODBC test suite against the universal driver

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SNOWFLAKE_TESTS="$PROJECT_ROOT/snowflake_tests"
PARAMS="$PROJECT_ROOT/parameters.json"
UNIVERSAL_DRIVER="$PROJECT_ROOT/target/release/libsfodbc.dylib"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Snowflake ODBC Test Suite - Universal Driver${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Build universal driver if needed
if [ ! -f "$UNIVERSAL_DRIVER" ]; then
    echo -e "${YELLOW}Building universal driver...${NC}"
    cd "$PROJECT_ROOT"
    cargo build -p odbc --release
    echo -e "${GREEN}✓ Driver built${NC}"
fi

# Extract connection parameters
echo -e "${BLUE}Loading connection parameters...${NC}"
ACCOUNT=$(grep "SNOWFLAKE_TEST_ACCOUNT" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
USER=$(grep "SNOWFLAKE_TEST_USER" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
PASSWORD=$(grep "SNOWFLAKE_TEST_PASSWORD\":" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
DATABASE=$(grep "SNOWFLAKE_TEST_DATABASE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
SCHEMA=$(grep "SNOWFLAKE_TEST_SCHEMA" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
WAREHOUSE=$(grep "SNOWFLAKE_TEST_WAREHOUSE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')
ROLE=$(grep "SNOWFLAKE_TEST_ROLE" "$PARAMS" | head -1 | sed 's/.*": "\(.*\)".*/\1/')

echo -e "${GREEN}✓ Parameters loaded${NC}"
echo -e "  Account: ${CYAN}$ACCOUNT${NC}"
echo -e "  User: ${CYAN}$USER${NC}"
echo -e "  Database: ${CYAN}$DATABASE${NC}"
echo ""

# Set environment variables for tests
export UNIVERSAL_DRIVER_PATH="$UNIVERSAL_DRIVER"
export SF_TEST_SERVER="${ACCOUNT}.snowflakecomputing.com"
export SF_TEST_ACCOUNT="$ACCOUNT"
export SF_TEST_USER="$USER"
export SF_TEST_PASSWORD="$PASSWORD"
export SF_TEST_DATABASE="$DATABASE"
export SF_TEST_SCHEMA="$SCHEMA"
export SF_TEST_WAREHOUSE="$WAREHOUSE"
export SF_TEST_ROLE="$ROLE"
export RUST_LOG=${RUST_LOG:-warn}

# Set DYLD_LIBRARY_PATH for odbcinst
export DYLD_LIBRARY_PATH="/opt/homebrew/opt/unixodbc/lib:${DYLD_LIBRARY_PATH:-}"

# Create a temporary ODBCINI file with a DSN that points to the universal driver
ODBC_DIR="$PROJECT_ROOT/.odbc"
mkdir -p "$ODBC_DIR"
ODBCINI_FILE="$ODBC_DIR/odbc_universal.ini"
cat > "$ODBCINI_FILE" <<EOF
[ODBC Data Sources]
universal_driver_dsn=UniversalSnowflakeDriver

[universal_driver_dsn]
Driver=$UNIVERSAL_DRIVER
SERVER=${SF_TEST_SERVER}
ACCOUNT=${ACCOUNT}
UID=${USER}
PWD=${PASSWORD}
DATABASE=${DATABASE}
SCHEMA=${SCHEMA}
WAREHOUSE=${WAREHOUSE}
ROLE=${ROLE}
EOF

export ODBCINI="$ODBCINI_FILE"
echo -e "${GREEN}✓ Created temporary DSN in ${ODBCINI}${NC}"

# Configure Simba ini so tests can locate log files without crashing
LOG_DIR="$ODBC_DIR/logs"
mkdir -p "$LOG_DIR"
SIMBAINI_FILE="$ODBC_DIR/simbaini.ini"
cat > "$SIMBAINI_FILE" <<EOF
[DriverManager]
LogPath=$LOG_DIR
EOF
touch "$LOG_DIR/snowflake_odbc_connection_0.log" "$LOG_DIR/snowflake_odbc_generic0.log"
export SIMBAINI="$SIMBAINI_FILE"
echo -e "${GREEN}✓ Configured Simba ini at ${SIMBAINI}${NC}"

# Adapt tests for universal driver
echo -e "${BLUE}Adapting tests for universal driver...${NC}"
cd "$SNOWFLAKE_TESTS"
python3 adapt_tests.py --driver universal
echo -e "${GREEN}✓ Tests adapted${NC}"
echo ""

# Build tests
echo -e "${BLUE}Building test suite...${NC}"

if [ ! -d "build" ]; then
    mkdir build
fi

cd build
cmake .. >/dev/null 2>&1 || {
    echo -e "${RED}✗ CMake configuration failed${NC}"
    cmake ..
    exit 1
}

echo -e "${YELLOW}Compiling tests (this may take a while)...${NC}"
make -j8 2>&1 | grep -E "(Building|Linking|error|warning:)" | head -20 || true

echo -e "${GREEN}✓ Tests built${NC}"
echo ""

# Run tests
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Running Test Suite${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create results directory
RESULTS_DIR="$PROJECT_ROOT/test_results"
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="$RESULTS_DIR/snowflake_tests_$TIMESTAMP.txt"

# Run CTest with output
echo -e "${CYAN}Running tests...${NC}"
echo ""

ctest --output-on-failure --timeout 1800 2>&1 | tee "$RESULTS_FILE"

# Parse results
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Test Results Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

TOTAL=$(grep -c "Test #" "$RESULTS_FILE" || echo "0")
PASSED=$(grep -c "Passed" "$RESULTS_FILE" || echo "0")
FAILED=$(grep -c "Failed" "$RESULTS_FILE" || echo "0")

if [ "$FAILED" -eq 0 ] && [ "$TOTAL" -gt 0 ]; then
    echo -e "${GREEN}✓ All $TOTAL tests passed!${NC}"
elif [ "$TOTAL" -gt 0 ]; then
    echo -e "  Total:  $TOTAL"
    echo -e "  ${GREEN}Passed: $PASSED${NC}"
    echo -e "  ${RED}Failed: $FAILED${NC}"
    
    echo ""
    echo -e "${YELLOW}Failed tests:${NC}"
    grep "Failed" "$RESULTS_FILE" | sed 's/^/  /'
else
    echo -e "${YELLOW}No tests were run${NC}"
fi

echo ""
echo -e "${CYAN}Full results saved to: $RESULTS_FILE${NC}"
echo ""

# Show pass rate
if [ "$TOTAL" -gt 0 ]; then
    PASS_RATE=$((PASSED * 100 / TOTAL))
    if [ "$PASS_RATE" -ge 90 ]; then
        COLOR=$GREEN
    elif [ "$PASS_RATE" -ge 70 ]; then
        COLOR=$YELLOW
    else
        COLOR=$RED
    fi
    echo -e "${COLOR}Pass Rate: $PASS_RATE%${NC}"
fi

echo ""

