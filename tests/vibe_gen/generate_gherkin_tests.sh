#!/bin/bash

# Script to generate Gherkin tests for ODBC functions using Claude Code
# Usage: ./generate_gherkin_tests.sh <ODBC_FUNCTION_NAME>
#
# Example: ./generate_gherkin_tests.sh SQLDriverConnect

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
JSON_FILE="${SCRIPT_DIR}/odbc_functions.json"
TEMPLATE_FILE="${SCRIPT_DIR}/gherkin_test_prompt_template.md"
OUTPUT_DIR="${ROOT_DIR}/tests/definitions/odbc/vibe"
LOG_FILE="${ROOT_DIR}/gherkin_generation.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print usage
usage() {
    echo -e "${BLUE}Usage:${NC} $0 <ODBC_FUNCTION_NAME>"
    echo ""
    echo "Arguments:"
    echo "  ODBC_FUNCTION_NAME    Name of the ODBC function (e.g., SQLDriverConnect)"
    echo ""
    echo "Example:"
    echo "  $0 SQLDriverConnect"
    echo ""
    echo "Available functions can be found in ${JSON_FILE}"
    exit 1
}

# Check dependencies
check_dependencies() {
    echo -e "${BLUE}Checking dependencies...${NC}"

    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error: jq is not installed. Please install it first.${NC}"
        echo "  Ubuntu/Debian: sudo apt-get install jq"
        echo "  macOS: brew install jq"
        exit 1
    fi

    if ! command -v sf &> /dev/null; then
        echo -e "${RED}Error: sf CLI is not installed or not in PATH.${NC}"
        exit 1
    fi

    if [ ! -f "$JSON_FILE" ]; then
        echo -e "${RED}Error: $JSON_FILE not found${NC}"
        exit 1
    fi

    if [ ! -f "$TEMPLATE_FILE" ]; then
        echo -e "${RED}Error: $TEMPLATE_FILE not found${NC}"
        exit 1
    fi
}

# Validate that the function exists in odbc_functions.json
validate_function() {
    local function_name="$1"
    
    echo -e "${BLUE}Validating function: ${function_name}...${NC}"
    
    # Check if function exists in JSON
    local exists=$(jq -r --arg name "$function_name" '.[] | select(.name == $name) | .name' "$JSON_FILE")
    
    if [ -z "$exists" ]; then
        echo -e "${RED}Error: Function '${function_name}' not found in ${JSON_FILE}${NC}"
        echo ""
        echo "Available functions:"
        jq -r '.[].name' "$JSON_FILE" | head -20
        echo "... (run 'jq -r '.[].name' ${JSON_FILE}' to see all)"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Function '${function_name}' is valid${NC}"
}

# Get function details from JSON
get_function_details() {
    local function_name="$1"
    
    # Get return type
    RETURN_TYPE=$(jq -r --arg name "$function_name" '.[] | select(.name == $name) | .return_type' "$JSON_FILE")
    
    # Get parameters as formatted list
    PARAMETERS=$(jq -r --arg name "$function_name" '.[] | select(.name == $name) | .args[] | "- \(.name): \(.type)"' "$JSON_FILE")
}

# Generate the prompt from template
generate_prompt() {
    local function_name="$1"
    
    echo -e "${BLUE}Generating prompt for ${function_name}...${NC}"
    
    # Read template
    local template=$(cat "$TEMPLATE_FILE")
    
    # Replace placeholders
    local prompt="${template//\{\{FUNCTION_NAME\}\}/$function_name}"
    prompt="${prompt//\{\{RETURN_TYPE\}\}/$RETURN_TYPE}"
    
    # Replace parameters (handle newlines properly)
    # Using a temp file for multi-line replacement
    local temp_file=$(mktemp)
    echo "$prompt" > "$temp_file"
    
    # Create parameters temp file
    local params_file=$(mktemp)
    echo "$PARAMETERS" > "$params_file"
    
    # Use awk for multi-line replacement
    awk -v params="$(cat "$params_file")" '{gsub(/\{\{PARAMETERS\}\}/, params); print}' "$temp_file" > "${temp_file}.new"
    mv "${temp_file}.new" "$temp_file"
    
    RENDERED_PROMPT=$(cat "$temp_file")
    
    # Cleanup
    rm -f "$temp_file" "$params_file"
}

# Create prompt file and call Claude
run_claude() {
    local function_name="$1"
    
    # Create output directory for the function
    local function_dir="${OUTPUT_DIR}/${function_name}"
    mkdir -p "$function_dir"
    
    # Save rendered prompt
    local prompt_file="${function_dir}/prompt.md"
    echo "$RENDERED_PROMPT" > "$prompt_file"
    
    echo -e "${BLUE}Prompt saved to: ${prompt_file}${NC}"
    echo ""
    echo -e "${YELLOW}Running Claude AI to generate Gherkin tests...${NC}"
    echo ""
    
    # Call sf ai claude with the prompt
    if sf ai claude -- "Generate Gherkin tests based on this prompt: @${prompt_file}"; then
        echo ""
        echo -e "${GREEN}✓ Claude AI completed successfully${NC}"
        echo -e "${GREEN}Check ${OUTPUT_DIR}/${function_name}/ for generated tests${NC}"
    else
        echo ""
        echo -e "${RED}✗ Claude AI failed${NC}"
        exit 1
    fi
}

# Main execution
main() {
    # Check for argument
    if [ $# -lt 1 ]; then
        usage
    fi

    local function_name="$1"

    echo "========================================="
    echo "ODBC Gherkin Test Generator"
    echo "========================================="
    echo ""

    check_dependencies
    validate_function "$function_name"
    get_function_details "$function_name"
    
    echo ""
    echo -e "${BLUE}Function Details:${NC}"
    echo "  Name: $function_name"
    echo "  Return Type: $RETURN_TYPE"
    echo "  Parameters:"
    echo "$PARAMETERS" | sed 's/^/    /'
    echo ""
    
    generate_prompt "$function_name"
    run_claude "$function_name"
    
    echo ""
    echo "========================================="
    echo -e "${GREEN}Generation complete!${NC}"
    echo "========================================="
}

# Run main function
main "$@"
