#!/bin/bash

# Test Format Validator Runner
# Validates that Gherkin feature files have corresponding test implementations.
#
# Strategy:
# - If the release binary exists and is newer than every source file under src/ and
#   the Cargo.{toml,lock} manifests, exec it directly (no cargo overhead).
# - Otherwise, build once with `cargo build --release` and then exec.
#
# This keeps pre-commit fast: the steady-state cost is just the binary run time
# (~1–2s), instead of paying cargo's lockfile/fingerprint overhead every commit.

set -e

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
BINARY="$SCRIPT_DIR/target/release/tests_format_validator"

cd "$SCRIPT_DIR"

needs_build() {
    [ ! -x "$BINARY" ] && return 0
    # Rebuild if any source or manifest is newer than the binary.
    local newer
    newer=$(find src Cargo.toml Cargo.lock -type f -newer "$BINARY" -print -quit 2>/dev/null || true)
    [ -n "$newer" ]
}

if needs_build; then
    echo "🔨 Building tests_format_validator (release)..."
    cargo build --release --quiet
fi

exec "$BINARY" \
    --workspace "$PROJECT_ROOT" \
    --features "$PROJECT_ROOT/tests/definitions" \
    "$@"
