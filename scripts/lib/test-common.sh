# shellcheck shell=bash
# Shared helpers for scripts/test-*.sh and scripts/format.sh. Source, do not execute.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export REPO_ROOT

die() {
    echo "error: $*" >&2
    exit 1
}

run() {
    printf '+'
    printf ' %q' "$@"
    printf '\n'
    "$@"
}

native_lib_name() {
    case "$(uname -s)" in
        Darwin) echo "lib$1.dylib" ;;
        Linux) echo "lib$1.so" ;;
        *) die "unsupported OS $(uname -s); these scripts cover macOS and Linux only" ;;
    esac
}

ensure_cargo() {
    if ! command -v cargo >/dev/null 2>&1 && [[ -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    command -v cargo >/dev/null 2>&1 || die "cargo not found; install the Rust toolchain (https://rustup.rs)"
}

cargo_target_dir() {
    echo "${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
}

require_tool() {
    command -v "$1" >/dev/null 2>&1 || die "$1 not found on PATH${2:+; $2}"
}

require_parameters() {
    export PARAMETER_PATH="${PARAMETER_PATH:-$REPO_ROOT/parameters.json}"
    [[ -f "$PARAMETER_PATH" ]] \
        || die "credentials file $PARAMETER_PATH not found; run ./scripts/decode_secrets.sh from the repo root, or set PARAMETER_PATH"
}

# Usage: check_suite <usage-fn> "$SUITE" <valid-suites...>
check_suite() {
    local usage_fn="$1" suite="$2"
    shift 2
    if [[ -z "$suite" ]]; then
        "$usage_fn" >&2
        exit 2
    fi
    local s
    for s in "$@"; do
        [[ "$s" == "$suite" ]] && return 0
    done
    echo "error: unknown suite '$suite' (expected one of: $*)" >&2
    exit 2
}
