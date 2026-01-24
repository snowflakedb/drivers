#!/bin/bash

# Characterization Test Generator
# This script generates characterization tests for Snowflake type to SQL C type conversions.
# It creates a new branch in a separate worktree, invokes Claude Code with a specially prepared
# prompt, and generates tests that characterize the OLD ODBC driver's behavior.
#
# Usage: ./scripts/generate_characterization_tests.sh <SNOWFLAKE_TYPE> <SQL_C_TYPE>
# Example: ./scripts/generate_characterization_tests.sh VARCHAR SQL_C_NUMERIC
#
# Environment variables:
#   PARENT_BRANCH - Parent branch to rebase on (default: NO-SNOW-characterization-tests)
#   CLAUDE_MODEL - Claude model to use (default: sonnet)
#   WORKTREE_BASE - Base directory for worktrees (default: parent of project root)
#   PARAMETER_PATH - Path to parameters.json file (default: $PROJECT_ROOT/parameters.json)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROMPT_TEMPLATE="$SCRIPT_DIR/characterization_prompt.md"

# Configuration
# Default parent branch for all characterization test branches
PARENT_BRANCH="${PARENT_BRANCH:-NO-SNOW-characterization-tests}"
CLAUDE_MODEL="${CLAUDE_MODEL:-sonnet}"
WORKTREE_BASE="${WORKTREE_BASE:-$(dirname "$PROJECT_ROOT")/universal-driver-wt}"
PARAMETER_PATH="${PARAMETER_PATH:-$PROJECT_ROOT/parameters.json}"

# Internal flag to indicate we're running inside tmux (set by the script itself)
INSIDE_TMUX_SESSION="${_CHARGEN_INSIDE_TMUX:-false}"

# Parse options
while [[ $# -gt 0 ]]; do
    case $1 in
        -*)
            echo -e "${RED}Error: Unknown option: $1${NC}"
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

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
    echo "The script runs in a new tmux session and creates a separate git worktree."
    echo ""
    echo "Arguments:"
    echo "  SNOWFLAKE_TYPE   The Snowflake data type (e.g., VARCHAR, NUMBER, DATE)"
    echo "  SQL_C_TYPE       The SQL C type to convert to (e.g., SQL_C_CHAR, SQL_C_NUMERIC)"
    echo ""
    echo "Environment variables:"
    echo "  PARENT_BRANCH    Parent branch to rebase on (default: NO-SNOW-characterization-tests)"
    echo "  CLAUDE_MODEL     Claude model to use (default: sonnet)"
    echo "  WORKTREE_BASE    Base directory for worktrees (default: parent of project root)"
    echo "  PARAMETER_PATH   Path to parameters.json file (default: \$PROJECT_ROOT/parameters.json)"
    echo ""
    echo "Valid Snowflake types:"
    echo "  ${VALID_SNOWFLAKE_TYPES[*]}"
    echo ""
    echo "Valid SQL C types:"
    echo "  ${VALID_SQL_C_TYPES[*]}"
    echo ""
    echo "Examples:"
    echo "  $0 VARCHAR SQL_C_NUMERIC"
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

# Check if tmux is available
if ! command -v tmux &> /dev/null; then
    echo -e "${RED}Error: tmux not found. Please install it first.${NC}"
    exit 1
fi

# Navigate to project root
cd "$PROJECT_ROOT"

# Create branch name (lowercase, replace underscores with dashes)
SNOWFLAKE_TYPE_LOWER=$(echo "$SNOWFLAKE_TYPE" | tr '[:upper:]' '[:lower:]')
SQL_C_TYPE_LOWER=$(echo "$SQL_C_TYPE" | tr '[:upper:]' '[:lower:]' | sed 's/_/-/g')
BRANCH_NAME="characterization/${SNOWFLAKE_TYPE_LOWER}-to-${SQL_C_TYPE_LOWER}"
TMUX_SESSION_NAME="chargen-${SNOWFLAKE_TYPE_LOWER}-${SQL_C_TYPE_LOWER}"
WORKTREE_DIR="$WORKTREE_BASE/universal-driver-${SNOWFLAKE_TYPE_LOWER}-to-${SQL_C_TYPE_LOWER}"

# Check if parameters.json exists
if [[ ! -f "$PARAMETER_PATH" ]]; then
    echo -e "${RED}Error: parameters.json not found at $PARAMETER_PATH${NC}"
    echo "Please set PARAMETER_PATH environment variable or create parameters.json in project root."
    exit 1
fi

# If not already inside tmux session started by this script, create one and re-run
if [[ "$INSIDE_TMUX_SESSION" != "true" ]]; then
    echo -e "${GREEN}=== Characterization Test Generator ===${NC}"
    echo "Snowflake Type: $SNOWFLAKE_TYPE"
    echo "SQL C Type: $SQL_C_TYPE"
    echo "Tmux Session: $TMUX_SESSION_NAME"
    echo "Parent Branch: $PARENT_BRANCH (will rebase on it)"
    echo "New Branch: $BRANCH_NAME"
    echo "Worktree: $WORKTREE_DIR"
    echo ""
    
    # Check if tmux session already exists
    if tmux has-session -t "$TMUX_SESSION_NAME" 2>/dev/null; then
        echo -e "${YELLOW}Tmux session '$TMUX_SESSION_NAME' already exists.${NC}"
        echo "To attach: tmux attach-session -t $TMUX_SESSION_NAME"
        read -p "Kill and recreate? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            tmux kill-session -t "$TMUX_SESSION_NAME"
        else
            echo "Keeping existing session."
            exit 0
        fi
    fi
    
    echo -e "${GREEN}Starting tmux session '$TMUX_SESSION_NAME'...${NC}"
    
    # Build the command to run inside tmux
    TMUX_CMD="_CHARGEN_INSIDE_TMUX=true"
    TMUX_CMD+=" PARENT_BRANCH='$PARENT_BRANCH'"
    TMUX_CMD+=" CLAUDE_MODEL='$CLAUDE_MODEL'"
    TMUX_CMD+=" WORKTREE_BASE='$WORKTREE_BASE'"
    TMUX_CMD+=" PARAMETER_PATH='$PARAMETER_PATH'"
    TMUX_CMD+=" '$SCRIPT_DIR/generate_characterization_tests.sh' '$SNOWFLAKE_TYPE' '$SQL_C_TYPE'"
    
    # Create new tmux session and run the script inside it
    # After script completes, cd to worktree directory and start a new bash shell
    tmux new-session -d -s "$TMUX_SESSION_NAME" -c "$PROJECT_ROOT" "bash -c '$TMUX_CMD; cd \"$WORKTREE_DIR\" && exec bash'"
    
    echo -e "${GREEN}Tmux session '$TMUX_SESSION_NAME' started in background.${NC}"
    read -p "Attach to session? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        tmux attach-session -t "$TMUX_SESSION_NAME"
    else
        echo "To attach later: tmux attach-session -t $TMUX_SESSION_NAME"
    fi
    exit 0
fi

# === From here on, we're running inside the tmux session ===

echo -e "${GREEN}=== Characterization Test Generator (in tmux) ===${NC}"
echo "Snowflake Type: $SNOWFLAKE_TYPE"
echo "SQL C Type: $SQL_C_TYPE"
echo "Parent Branch: $PARENT_BRANCH (rebasing on it)"
echo "New Branch: $BRANCH_NAME"
echo "Worktree: $WORKTREE_DIR"
echo ""

# Fetch latest from remote
echo -e "${GREEN}Fetching latest from remote...${NC}"
git fetch origin "$PARENT_BRANCH"

# Check if worktree already exists
if [[ -d "$WORKTREE_DIR" ]]; then
    echo -e "${YELLOW}Worktree directory $WORKTREE_DIR already exists.${NC}"
    read -p "Remove and recreate? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git worktree remove --force "$WORKTREE_DIR" 2>/dev/null || rm -rf "$WORKTREE_DIR"
    else
        echo "Aborted."
        exit 1
    fi
fi

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

# Create new branch and worktree
echo -e "${GREEN}Creating worktree at $WORKTREE_DIR with branch $BRANCH_NAME...${NC}"
git worktree add -b "$BRANCH_NAME" "$WORKTREE_DIR" "origin/$PARENT_BRANCH"

# Copy parameters.json to the new worktree
echo -e "${GREEN}Copying parameters.json to worktree...${NC}"
cp "$PARAMETER_PATH" "$WORKTREE_DIR/parameters.json"

# Change to the worktree directory
cd "$WORKTREE_DIR"

# Rebase on the parent branch to ensure we're up to date
echo -e "${GREEN}Rebasing on origin/$PARENT_BRANCH...${NC}"
git rebase "origin/$PARENT_BRANCH"

# Update PROJECT_ROOT for the worktree
PROJECT_ROOT="$WORKTREE_DIR"

# Prepare the prompt by replacing placeholders
echo -e "${GREEN}Preparing prompt...${NC}"
PROMPT_FILE="characterization_prompt_${SNOWFLAKE_TYPE}_to_${SQL_C_TYPE}.md" 
cat "$PROMPT_TEMPLATE" | \
    sed "s/{{SNOWFLAKE_TYPE}}/$SNOWFLAKE_TYPE/g" | \
    sed "s/{{SQL_C_TYPE}}/$SQL_C_TYPE/g" | tee "$PROMPT_FILE"

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
    "$PROMPT_FILE"

echo ""
echo -e "${GREEN}=== Generation Complete ===${NC}"
