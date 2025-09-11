#!/bin/bash

# Test Format Validator Runner
# Validates that Gherkin feature files have corresponding test implementations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

echo "🔍 Running tests format validator..."
echo "Project root: $PROJECT_ROOT"

cd "$SCRIPT_DIR"

# Run the validator with project-specific paths
cargo run -- \
    --workspace "$PROJECT_ROOT" \
    --features "$PROJECT_ROOT/tests/e2e"


