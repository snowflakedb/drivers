#!/bin/bash -e
#
# Test certificate revocation validation using the revocation-validation framework.
#

set -o pipefail
set +x

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
DRIVER_ROOT="$( dirname "${THIS_DIR}")"
WORKSPACE="${WORKSPACE:-${DRIVER_ROOT}}"

echo "[Info] Starting revocation validation tests"

# Ensure unzip is available (needed by protoc installer during cargo build)
if ! command -v unzip >/dev/null 2>&1; then
    echo "[Info] Installing unzip..."
    yum install -y unzip || apt-get install -y unzip || true
fi

# Clone revocation-validation framework.
# REVOCATION_REF accepts a branch, tag, or commit SHA (git --branch supports all three).
# Legacy env var `REVOCATION_BRANCH` is still honoured for backward compatibility.
# Production CI should pass a pinned tag or commit SHA (e.g. REVOCATION_REF=v1.2.3) so
# test results are reproducible over time. Falling back to `main` is convenient for
# local runs but makes runs non-deterministic if the upstream repo changes.
REVOCATION_REF="${REVOCATION_REF:-${REVOCATION_BRANCH:-main}}"
REVOCATION_REPO="https://github.com/snowflake-eng/revocation-validation.git"
REVOCATION_DIR="$(mktemp -d "${TMPDIR:-/tmp}/revocation-validation.XXXXXX")"

if [ "$REVOCATION_REF" = "main" ]; then
    echo "[Warn] REVOCATION_REF defaulted to 'main' — results are non-reproducible." >&2
    echo "[Warn] Production CI should set REVOCATION_REF to a pinned tag or commit SHA." >&2
fi

# Clean up workspace AND any ASKPASS helper on exit. The askpass script is chmod 700
# and deleted here to minimize the window in which the helper file exists on disk.
ASKPASS_SCRIPT=""
cleanup() {
    rm -rf "$REVOCATION_DIR"
    if [ -n "$ASKPASS_SCRIPT" ]; then
        rm -f "$ASKPASS_SCRIPT"
    fi
}
trap cleanup EXIT

if [ -n "$GITHUB_USER" ] && [ -n "$GITHUB_TOKEN" ]; then
    # Use GIT_ASKPASS instead of embedding the token in the clone URL. Tokens in URLs
    # leak via `ps` output, shell history, git error messages, and sometimes CI build
    # logs. GIT_ASKPASS is the canonical git auth mechanism for non-interactive use:
    # git invokes the helper to get the password, which we read from the env (passed
    # down to the helper process, not exposed on any command line).
    ASKPASS_SCRIPT="$(mktemp "${TMPDIR:-/tmp}/git-askpass.XXXXXX")"
    cat >"$ASKPASS_SCRIPT" <<'EOF'
#!/bin/sh
# git calls this helper with a prompt like "Username for ..." or "Password for ...".
# Match on the first word to return the right credential from the env.
case "$1" in
    Username*) printf '%s\n' "$GITHUB_USER" ;;
    *)         printf '%s\n' "$GITHUB_TOKEN" ;;
esac
EOF
    chmod 700 "$ASKPASS_SCRIPT"
    GIT_ASKPASS="$ASKPASS_SCRIPT" GIT_TERMINAL_PROMPT=0 \
        git clone -q --depth 1 --branch "$REVOCATION_REF" \
        "$REVOCATION_REPO" "$REVOCATION_DIR"
else
    git clone -q --depth 1 --branch "$REVOCATION_REF" "$REVOCATION_REPO" "$REVOCATION_DIR"
fi

# Scrub GitHub credentials from the environment BEFORE executing any code from the
# cloned external repo. `go run .` below compiles and runs third-party Go code that
# could read os.Getenv or spawn subprocesses which inherit our env. Keeping the token
# alive past the git clone gives unnecessary exposure. Also blank the ASKPASS script
# now (it's already chmod 700 and will be removed in the EXIT trap, but a pre-run
# `: > ...` truncates the contents immediately, eliminating the window in which the
# file contains credentials on disk).
unset GITHUB_USER GITHUB_TOKEN
if [ -n "$ASKPASS_SCRIPT" ]; then
    : > "$ASKPASS_SCRIPT"
fi

cd "$REVOCATION_DIR"

echo "[Info] Running tests with Go $(go version | grep -oE 'go[0-9]+\.[0-9]+')..."

set +e
go run . \
    --client universal-driver-rust \
    --universal-driver-path "$DRIVER_ROOT" \
    --output "${WORKSPACE}/revocation-results.json" \
    --output-html "${WORKSPACE}/revocation-report.html" \
    --log-level debug
EXIT_CODE=$?
set -e

if [ -f "${WORKSPACE}/revocation-results.json" ] && [ -f "${WORKSPACE}/revocation-report.html" ]; then
    echo "[Info] Results: ${WORKSPACE}/revocation-results.json"
    echo "[Info] Report: ${WORKSPACE}/revocation-report.html"
else
    echo "[Warn] Expected output files were not generated"
fi

exit $EXIT_CODE
