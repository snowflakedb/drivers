#!/bin/bash

# Script to generate Gherkin and CATCH tests for ODBC functions, commit changes, and submit a PR
# Usage: ./generate_tests_and_submit_pr.sh <ODBC_FUNCTION_NAME> [--dry-run]
#
# Example: ./generate_tests_and_submit_pr.sh SQLDriverConnect
#          ./generate_tests_and_submit_pr.sh SQLDriverConnect --dry-run

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GHERKIN_SCRIPT="${SCRIPT_DIR}/generate_gherkin_tests.sh"
CATCH_SCRIPT="${SCRIPT_DIR}/generate_catch_tests.sh"
BASE_BRANCH="main"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Flags
DRY_RUN=false

# Print usage
usage() {
    echo -e "${BLUE}Usage:${NC} $0 <ODBC_FUNCTION_NAME> [--dry-run]"
    echo ""
    echo "Arguments:"
    echo "  ODBC_FUNCTION_NAME    Name of the ODBC function (e.g., SQLDriverConnect)"
    echo ""
    echo "Options:"
    echo "  --dry-run             Generate tests but skip git commit and PR submission"
    echo ""
    echo "Example:"
    echo "  $0 SQLDriverConnect"
    echo "  $0 SQLDriverConnect --dry-run"
    exit 1
}

# Log functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}Step: $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Check dependencies
check_dependencies() {
    log_step "Checking dependencies"
    
    local missing_deps=()
    
    if ! command -v git &> /dev/null; then
        missing_deps+=("git")
    fi
    
    if ! command -v gh &> /dev/null; then
        missing_deps+=("gh (GitHub CLI)")
    fi
    
    if [ ! -x "$GHERKIN_SCRIPT" ]; then
        log_error "Gherkin test script not found or not executable: $GHERKIN_SCRIPT"
        exit 1
    fi
    
    if [ ! -x "$CATCH_SCRIPT" ]; then
        log_error "CATCH test script not found or not executable: $CATCH_SCRIPT"
        exit 1
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        echo "Please install the missing dependencies:"
        echo "  - git: Required for version control"
        echo "  - gh: GitHub CLI for creating PRs (https://cli.github.com/)"
        exit 1
    fi
    
    # Check if gh is authenticated
    if ! gh auth status &> /dev/null; then
        log_error "GitHub CLI is not authenticated. Please run 'gh auth login' first."
        exit 1
    fi
    
    log_success "All dependencies are available"
}

# Check git repository status
check_git_status() {
    log_step "Checking git repository status"
    
    # Ensure we're in a git repository
    if ! git rev-parse --is-inside-work-tree &> /dev/null; then
        log_error "Not inside a git repository"
        exit 1
    fi
    
    # Check for uncommitted changes
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
        log_warning "You have uncommitted changes in your working directory"
        echo "Please commit or stash your changes before running this script."
        exit 1
    fi
    
    # Fetch latest from remote
    log_info "Fetching latest changes from remote..."
    git fetch origin
    
    log_success "Git repository is clean and ready"
}

# Create feature branch
create_branch() {
    local function_name="$1"
    local timestamp=$(date +%Y%m%d-%H%M%S)
    BRANCH_NAME="tests/${function_name,,}-${timestamp}"
    
    log_step "Creating feature branch"
    
    log_info "Checking out ${BASE_BRANCH}..."
    git checkout "${BASE_BRANCH}"
    
    log_info "Pulling latest changes..."
    git pull origin "${BASE_BRANCH}"
    
    log_info "Creating branch: ${BRANCH_NAME}"
    git checkout -b "${BRANCH_NAME}"
    
    log_success "Created and switched to branch: ${BRANCH_NAME}"
}

# Generate Gherkin tests
generate_gherkin_tests() {
    local function_name="$1"
    
    log_step "Generating Gherkin tests for ${function_name}"
    
    if "$GHERKIN_SCRIPT" "$function_name"; then
        log_success "Gherkin tests generated successfully"
    else
        log_error "Failed to generate Gherkin tests"
        exit 1
    fi
}

# Generate CATCH tests
generate_catch_tests() {
    local function_name="$1"
    
    log_step "Generating CATCH tests for ${function_name}"
    
    if "$CATCH_SCRIPT" "$function_name"; then
        log_success "CATCH tests generated successfully"
    else
        log_error "Failed to generate CATCH tests"
        exit 1
    fi
}

# Commit changes
commit_changes() {
    local function_name="$1"
    
    log_step "Committing changes"
    
    # Add all generated test files
    git add "${ROOT_DIR}/tests/definitions/odbc/vibe/${function_name}/" 2>/dev/null || true
    git add "${ROOT_DIR}/odbc_tests/tests/vibe/${function_name}/" 2>/dev/null || true
    
    # Check if there are changes to commit
    if git diff --cached --quiet; then
        log_warning "No changes to commit"
        return 1
    fi
    
    # Show what will be committed
    log_info "Files to be committed:"
    git diff --cached --name-only | sed 's/^/  /'
    
    # Commit with a descriptive message
    local commit_message="Add generated tests for ${function_name}

This commit includes:
- Gherkin test definitions for ${function_name}
- CATCH C++ test implementations for ${function_name}

Generated using generate_tests_and_submit_pr.sh"
    
    git commit -m "$commit_message"
    
    log_success "Changes committed successfully"
}

# Push branch and create PR
submit_pr() {
    local function_name="$1"
    
    log_step "Pushing branch and creating PR"
    
    log_info "Pushing branch to origin..."
    git push -u origin "${BRANCH_NAME}"
    
    log_info "Creating pull request..."
    
    local pr_title="Add generated tests for ${function_name}"
    local pr_body="## Description

This PR adds automatically generated tests for the ODBC function \`${function_name}\`.

## Changes

- **Gherkin Tests**: Added behavior specifications in \`tests/definitions/odbc/vibe/${function_name}/\`
- **CATCH Tests**: Added C++ test implementations in \`odbc_tests/tests/vibe/${function_name}/\`

## Generation

These tests were generated using the automated test generation pipeline:
1. \`generate_gherkin_tests.sh\` - Generated Gherkin feature files
2. \`generate_catch_tests.sh\` - Generated CATCH C++ tests

## Checklist

- [ ] Review generated Gherkin scenarios for correctness
- [ ] Review generated CATCH tests for correctness
- [ ] Verify tests compile successfully
- [ ] Run tests locally to ensure they pass"

    if gh pr create \
        --title "$pr_title" \
        --body "$pr_body" \
        --base "${BASE_BRANCH}" \
        --head "${BRANCH_NAME}"; then
        log_success "Pull request created successfully"
    else
        log_error "Failed to create pull request"
        exit 1
    fi
}

# Cleanup on failure
cleanup_on_failure() {
    log_warning "Cleaning up after failure..."
    
    # Try to go back to the base branch
    git checkout "${BASE_BRANCH}" 2>/dev/null || true
    
    # Delete the feature branch if it exists
    if [ -n "${BRANCH_NAME:-}" ]; then
        git branch -D "${BRANCH_NAME}" 2>/dev/null || true
    fi
}

# Parse arguments
parse_args() {
    if [ $# -lt 1 ]; then
        usage
    fi
    
    FUNCTION_NAME=""
    
    for arg in "$@"; do
        case $arg in
            --dry-run)
                DRY_RUN=true
                ;;
            -h|--help)
                usage
                ;;
            -*)
                log_error "Unknown option: $arg"
                usage
                ;;
            *)
                if [ -z "$FUNCTION_NAME" ]; then
                    FUNCTION_NAME="$arg"
                else
                    log_error "Multiple function names provided"
                    usage
                fi
                ;;
        esac
    done
    
    if [ -z "$FUNCTION_NAME" ]; then
        log_error "No function name provided"
        usage
    fi
}

# Main execution
main() {
    parse_args "$@"
    
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║     ODBC Test Generator and PR Submission Pipeline            ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  Function:  ${GREEN}${FUNCTION_NAME}${NC}"
    echo -e "  Dry Run:   ${YELLOW}${DRY_RUN}${NC}"
    echo ""
    
    # Set up trap for cleanup on failure
    trap cleanup_on_failure ERR
    
    check_dependencies
    
    if [ "$DRY_RUN" = false ]; then
        check_git_status
        create_branch "$FUNCTION_NAME"
    fi
    
    generate_gherkin_tests "$FUNCTION_NAME"
    generate_catch_tests "$FUNCTION_NAME"
    
    if [ "$DRY_RUN" = true ]; then
        log_step "Dry run complete"
        log_warning "Skipping commit and PR submission (dry-run mode)"
        log_info "Generated files are in your working directory"
    else
        if commit_changes "$FUNCTION_NAME"; then
            submit_pr "$FUNCTION_NAME"
            
            echo ""
            echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
            echo -e "${GREEN}║                    Pipeline Complete!                         ║${NC}"
            echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
            echo ""
            echo "  Branch: ${BRANCH_NAME}"
            echo "  Please review the PR and merge when ready."
            echo ""
        else
            log_warning "No changes were generated, skipping PR submission"
        fi
    fi
    
    # Remove the trap
    trap - ERR
}

# Run main function
main "$@"

