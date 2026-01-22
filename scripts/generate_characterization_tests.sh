#!/bin/bash

# Characterization Test Generator
# This script generates characterization tests for Snowflake type to SQL C type conversions.
# It creates a new branch, invokes Claude Code with a specially prepared prompt, and generates
# tests that characterize the OLD ODBC driver's behavior.
#
# Usage: ./scripts/generate_characterization_tests.sh <SNOWFLAKE_TYPE> <SQL_C_TYPE>
# Example: ./scripts/generate_characterization_tests.sh VARCHAR SQL_C_NUMERIC
#
# Environment variables:
#   PARENT_BRANCH - Parent branch to create characterization branch from (default: NO-SNOW-characterization-tests)
#   CLAUDE_MODEL - Claude model to use (default: sonnet)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROMPT_TEMPLATE="$SCRIPT_DIR/characterization_prompt.md"

# Configuration
# Default parent branch for all characterization test branches
PARENT_BRANCH="${PARENT_BRANCH:-NO-SNOW-characterization-tests}"
CLAUDE_MODEL="${CLAUDE_MODEL:-sonnet}"

# Valid Snowflake types
VALID_SNOWFLAKE_TYPES=(
    # String types
    "VARCHAR" "CHAR" "TEXT"
    # Numeric types
    "NUMBER" "DECIMAL" "NUMERIC" "INT" "INTEGER" "BIGINT" "SMALLINT" "TINYINT"
    # Floating point types
    "FLOAT" "REAL" "DOUBLE"
    # Temporal types
    "DATE" "TIME" "TIMESTAMP" "TIMESTAMP_NTZ" "TIMESTAMP_LTZ" "TIMESTAMP_TZ"
    # Boolean
    "BOOLEAN"
    # Binary types
    "BINARY" "VARBINARY"
    # Semi-structured types
    "VARIANT" "OBJECT" "ARRAY"
)

# Valid SQL C types
VALID_SQL_C_TYPES=(
    # Integer types
    "SQL_C_LONG" "SQL_C_SLONG" "SQL_C_ULONG"
    "SQL_C_SHORT" "SQL_C_SSHORT" "SQL_C_USHORT"
    "SQL_C_TINYINT" "SQL_C_STINYINT" "SQL_C_UTINYINT"
    "SQL_C_SBIGINT" "SQL_C_UBIGINT"
    # Float types
    "SQL_C_FLOAT" "SQL_C_DOUBLE"
    # String types
    "SQL_C_CHAR" "SQL_C_WCHAR" "SQL_C_BINARY"
    # Numeric
    "SQL_C_NUMERIC"
    # Temporal types
    "SQL_C_TYPE_DATE" "SQL_C_TYPE_TIME" "SQL_C_TYPE_TIMESTAMP"
    # Other types
    "SQL_C_BIT" "SQL_C_GUID"
    # Interval types
    "SQL_C_INTERVAL_YEAR" "SQL_C_INTERVAL_MONTH" "SQL_C_INTERVAL_DAY"
    "SQL_C_INTERVAL_HOUR" "SQL_C_INTERVAL_MINUTE" "SQL_C_INTERVAL_SECOND"
    "SQL_C_INTERVAL_YEAR_TO_MONTH" "SQL_C_INTERVAL_DAY_TO_HOUR"
    "SQL_C_INTERVAL_DAY_TO_MINUTE" "SQL_C_INTERVAL_DAY_TO_SECOND"
    "SQL_C_INTERVAL_HOUR_TO_MINUTE" "SQL_C_INTERVAL_HOUR_TO_SECOND"
    "SQL_C_INTERVAL_MINUTE_TO_SECOND"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_usage() {
    echo "Usage: $0 <SNOWFLAKE_TYPE> <SQL_C_TYPE>"
    echo ""
    echo "Generate characterization tests for Snowflake type to SQL C type conversion."
    echo ""
    echo "Arguments:"
    echo "  SNOWFLAKE_TYPE   The Snowflake data type (e.g., VARCHAR, NUMBER, DATE)"
    echo "  SQL_C_TYPE       The SQL C type to convert to (e.g., SQL_C_CHAR, SQL_C_NUMERIC)"
    echo ""
    echo "Environment variables:"
    echo "  PARENT_BRANCH    Parent branch to create from (default: NO-SNOW-characterization-tests)"
    echo "  CLAUDE_MODEL     Claude model to use (default: sonnet)"
    echo ""
    echo "Valid Snowflake types:"
    echo "  ${VALID_SNOWFLAKE_TYPES[*]}"
    echo ""
    echo "Valid SQL C types:"
    echo "  ${VALID_SQL_C_TYPES[*]}"
    echo ""
    echo "Examples:"
    echo "  $0 VARCHAR SQL_C_NUMERIC"
    echo "  $0 NUMBER SQL_C_CHAR"
    echo "  PARENT_BRANCH=develop $0 DATE SQL_C_TYPE_DATE"
}

is_valid_snowflake_type() {
    local type="$1"
    for valid_type in "${VALID_SNOWFLAKE_TYPES[@]}"; do
        if [[ "$type" == "$valid_type" ]]; then
            return 0
        fi
    done
    return 1
}

is_valid_sql_c_type() {
    local type="$1"
    for valid_type in "${VALID_SQL_C_TYPES[@]}"; do
        if [[ "$type" == "$valid_type" ]]; then
            return 0
        fi
    done
    return 1
}

# Check arguments
if [[ $# -lt 2 ]]; then
    echo -e "${RED}Error: Missing arguments${NC}"
    print_usage
    exit 1
fi

SNOWFLAKE_TYPE="$1"
SQL_C_TYPE="$2"

# Validate Snowflake type
if ! is_valid_snowflake_type "$SNOWFLAKE_TYPE"; then
    echo -e "${RED}Error: Invalid Snowflake type: $SNOWFLAKE_TYPE${NC}"
    echo "Valid types: ${VALID_SNOWFLAKE_TYPES[*]}"
    exit 1
fi

# Validate SQL C type
if ! is_valid_sql_c_type "$SQL_C_TYPE"; then
    echo -e "${RED}Error: Invalid SQL C type: $SQL_C_TYPE${NC}"
    echo "Valid types: ${VALID_SQL_C_TYPES[*]}"
    exit 1
fi

# Check if prompt template exists
if [[ ! -f "$PROMPT_TEMPLATE" ]]; then
    echo -e "${RED}Error: Prompt template not found: $PROMPT_TEMPLATE${NC}"
    exit 1
fi

# Check if claude CLI is available
if ! command -v claude &> /dev/null; then
    echo -e "${RED}Error: Claude CLI not found. Please install it first.${NC}"
    exit 1
fi

# Navigate to project root
cd "$PROJECT_ROOT"

# Create branch name (lowercase, replace underscores with dashes)
SNOWFLAKE_TYPE_LOWER=$(echo "$SNOWFLAKE_TYPE" | tr '[:upper:]' '[:lower:]')
SQL_C_TYPE_LOWER=$(echo "$SQL_C_TYPE" | tr '[:upper:]' '[:lower:]' | sed 's/_/-/g')
BRANCH_NAME="characterization/${SNOWFLAKE_TYPE_LOWER}-to-${SQL_C_TYPE_LOWER}"

echo -e "${GREEN}=== Characterization Test Generator ===${NC}"
echo "Snowflake Type: $SNOWFLAKE_TYPE"
echo "SQL C Type: $SQL_C_TYPE"
echo "Parent Branch: $PARENT_BRANCH"
echo "New Branch: $BRANCH_NAME"
echo ""

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${YELLOW}Warning: You have uncommitted changes.${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
fi

# Fetch latest from remote
echo -e "${GREEN}Fetching latest from remote...${NC}"
git fetch origin "$PARENT_BRANCH"

# Check if branch already exists
if git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
    echo -e "${YELLOW}Branch $BRANCH_NAME already exists.${NC}"
    read -p "Delete and recreate? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git branch -D "$BRANCH_NAME"
    else
        echo "Aborted."
        exit 1
    fi
fi

# Create new branch from parent
echo -e "${GREEN}Creating branch $BRANCH_NAME from origin/$PARENT_BRANCH...${NC}"
git checkout -b "$BRANCH_NAME" "origin/$PARENT_BRANCH"

# Prepare the prompt by replacing placeholders
echo -e "${GREEN}Preparing prompt...${NC}"
PROMPT=$(cat "$PROMPT_TEMPLATE" | \
    sed "s/{{SNOWFLAKE_TYPE}}/$SNOWFLAKE_TYPE/g" | \
    sed "s/{{SQL_C_TYPE}}/$SQL_C_TYPE/g")

# Create test directory if it doesn't exist
TEST_DIR="odbc_tests/tests/characterization/conversion"
mkdir -p "$TEST_DIR"

# Invoke Claude Code
echo -e "${GREEN}Invoking Claude Code...${NC}"
echo "Model: $CLAUDE_MODEL"
echo ""

# Run claude with the prompt
# Using --print for non-interactive mode
# Using --dangerously-skip-permissions to allow file operations without prompts
claude --model "$CLAUDE_MODEL" \
    --dangerously-skip-permissions \
    -p "$PROMPT"

echo ""
echo -e "${GREEN}=== Generation Complete ===${NC}"
echo "Branch: $BRANCH_NAME"
echo "Test directory: $TEST_DIR"
echo ""
echo "Next steps:"
echo "1. Review the generated tests"
echo "2. Build the tests:"
echo "   cd odbc_tests && cmake -B cmake-build -DDRIVER_TYPE=OLD . && cmake --build cmake-build"
echo "3. Run characterization tests against OLD driver:"
echo "   RUN_CHARACTERIZATION=1 ctest --test-dir cmake-build -R characterization --output-on-failure"
echo "   (Note: characterization tests are skipped by default unless RUN_CHARACTERIZATION=1)"
echo "4. Fix any issues and iterate"
echo "5. Commit and create PR"
