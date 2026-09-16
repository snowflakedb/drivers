#!/usr/bin/env bash
#
# Regenerate sf_core/include/sf_core_providers.h from sf_core/src/xp_backend/ffi.rs.
#
# The header is committed rather than generated at build time. Its consumer is
# Snowflake's execution platform, a C++ Bazel build with no Rust toolchain, so it
# needs the header as a source file; and generating it from build.rs would put
# cbindgen in the dependency graph of every client build that will never use it.
#
# Run this after changing anything in src/xp_backend/ffi.rs, and commit the result.
# Pass --check to verify the committed header is current without rewriting it
# (for CI).
#
# Usage:
#   scripts/generate_providers_header.sh [--check]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG="${REPO_ROOT}/sf_core/cbindgen-xp-providers.toml"
OUTPUT="${REPO_ROOT}/sf_core/include/sf_core_providers.h"

# One file rather than the crate: the header should describe only this ABI, and a
# whole-crate parse panics on unrelated generics elsewhere in sf_core
# ("DatabaseDriverClient has 0 params but is being instantiated with 1 values").
SOURCE="${REPO_ROOT}/sf_core/src/xp_backend/ffi.rs"

# `cargo install` lands in ~/.cargo/bin, not always on PATH in a nix shell.
if ! command -v cbindgen &>/dev/null; then
    export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
fi
if ! command -v cbindgen &>/dev/null; then
    echo "error: cbindgen not found. Install it with:" >&2
    echo "  cargo install cbindgen --version '^0.29' --locked" >&2
    echo >&2
    echo "0.29 or newer is required: earlier releases cannot parse the Rust 2024" >&2
    echo "edition's #[unsafe(no_mangle)] attribute that src/xp_backend/ffi.rs uses." >&2
    exit 1
fi

generated="$(mktemp)"
trap 'rm -f "${generated}"' EXIT

cbindgen --config "${CONFIG}" --output "${generated}" --quiet "${SOURCE}"

if [[ "${1:-}" == "--check" ]]; then
    if ! diff -u "${OUTPUT}" "${generated}"; then
        echo >&2
        echo "error: ${OUTPUT#"${REPO_ROOT}/"} is out of date." >&2
        echo "       Run scripts/generate_providers_header.sh and commit the result." >&2
        exit 1
    fi
    echo "sf_core_providers.h is up to date."
    exit 0
fi

mkdir -p "$(dirname "${OUTPUT}")"
cp "${generated}" "${OUTPUT}"
echo "wrote ${OUTPUT#"${REPO_ROOT}/"}"
