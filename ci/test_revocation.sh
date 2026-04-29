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

# Single point of EXIT cleanup. All cleanup targets are declared up-front so this
# function is the *only* place that knows how to dispose of them — adding a new
# tmp resource means: declare its variable here, populate it where needed, and
# extend cleanup() with a guarded removal. Avoid the antipattern of installing
# multiple `trap ... EXIT` handlers that overwrite each other; bash only keeps
# the most recent one, which historically caused us to silently drop earlier
# cleanup steps when reordering code.
#
# Tracking variables — declared empty so cleanup() can safely guard with `[ -n ]`
# even if the script exits before they're populated:
#   REVOCATION_DIR  — temp clone of the revocation-validation framework
#   ASKPASS_SCRIPT  — temp git-askpass helper (chmod 700; deleted to minimise
#                     the window in which the file exists on disk with creds)
#   CARGO_SHIM_DIR  — temp dir hosting the cargo wrapper that injects --features cli
ASKPASS_SCRIPT=""
CARGO_SHIM_DIR=""
cleanup() {
    [ -n "$REVOCATION_DIR" ] && rm -rf "$REVOCATION_DIR"
    [ -n "$ASKPASS_SCRIPT" ] && rm -f "$ASKPASS_SCRIPT"
    [ -n "$CARGO_SHIM_DIR" ] && rm -rf "$CARGO_SHIM_DIR"
    return 0
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

# ─── Cargo shim ────────────────────────────────────────────────────────────────
# PR snowflakedb/universal-driver#902 (Apr 2026) made clap an optional dep gated
# behind a `cli` feature, with `required-features = ["cli"]` on the tls_client and
# collect_chunk_data binaries. This shaved ~17% off sf_core's library binary size
# but broke the implicit contract that `cargo build --bin tls_client` works
# without feature flags — which is what the external revocation-validation
# framework below does.
#
# Rather than revert the binary-size win or patch the external framework, we
# prepend a shim `cargo` to PATH that auto-injects `--features cli` whenever the
# framework builds/runs one of sf_core's diagnostic binaries. The shim is local
# to this script; it does not affect anything outside this CI step.
#
# This workaround can be removed once either:
#   - snowflake-eng/revocation-validation supports passing cargo features, or
#   - the cli feature is re-promoted to default in sf_core's Cargo.toml.
CARGO_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cargo-shim.XXXXXX")"
REAL_CARGO="$(command -v cargo)"
if [ -z "$REAL_CARGO" ]; then
    echo "[Error] cargo not found on PATH — cannot set up compatibility shim" >&2
    exit 1
fi
# CARGO_SHIM_DIR is now populated; cleanup() picks it up automatically on EXIT
# (see the consolidated cleanup definition near the top of this script).
cat > "$CARGO_SHIM_DIR/cargo" <<EOF
#!/bin/bash
# Shim: auto-enable \`cli\` feature when building sf_core's diagnostic binaries.
# See surrounding comment in ci/test_revocation.sh for rationale.
set -e
if [[ "\$1" == "build" || "\$1" == "run" || "\$1" == "check" ]]; then
    needs_cli=false
    for arg in "\$@"; do
        if [[ "\$arg" == "tls_client" || "\$arg" == "collect_chunk_data" ]]; then
            needs_cli=true
            break
        fi
    done
    if [[ "\$needs_cli" == "true" ]]; then
        # Insert --features cli BEFORE any \`--\` separator so it's parsed as a cargo
        # flag, not passed to the built binary (matters for \`cargo run\`).
        new_args=()
        inserted=false
        for arg in "\$@"; do
            if [[ "\$arg" == "--" && "\$inserted" == "false" ]]; then
                new_args+=("--features" "cli" "--")
                inserted=true
            else
                new_args+=("\$arg")
            fi
        done
        if [[ "\$inserted" == "false" ]]; then
            new_args+=("--features" "cli")
        fi
        exec "$REAL_CARGO" "\${new_args[@]}"
    fi
fi
exec "$REAL_CARGO" "\$@"
EOF
chmod +x "$CARGO_SHIM_DIR/cargo"
export PATH="$CARGO_SHIM_DIR:$PATH"
echo "[Info] Installed cargo shim at $CARGO_SHIM_DIR (auto-enables cli feature for sf_core bins)"

echo "[Info] Running tests with Go $(go version | grep -oE 'go[0-9]+\.[0-9]+')..."

# Output policy:
# - DEFAULT (info-level): generate NO reports. The framework's stdout (captured in
#   the Jenkins console log) plus the exit code are the entire audit trail. No
#   files written to the workspace, nothing for Jenkins to archive or publish.
#   This is the lightest possible footprint and matches the steady-state CI need:
#   "did revocation validation pass?" → see the green stage in Jenkins; nothing
#   else to look at.
# - DEBUG (REVOCATION_LOG_LEVEL=debug): generate JSON + HTML reports to the
#   workspace AND keep them. Devs investigating a specific failure can set this
#   env var on a one-off Jenkins re-run (or local run) and inspect the artifacts
#   via the Jenkins workspace browser. Reports are still NOT archived — they live
#   in the build's workspace until the next build overwrites it.
LOG_LEVEL="${REVOCATION_LOG_LEVEL:-info}"

GO_RUN_ARGS=(
    --client universal-driver-rust
    --universal-driver-path "$DRIVER_ROOT"
    --log-level "$LOG_LEVEL"
)

if [ "$LOG_LEVEL" = "debug" ]; then
    echo "[Info] Debug mode — will write revocation-results.json and revocation-report.html to \$WORKSPACE."
    GO_RUN_ARGS+=(
        --output "${WORKSPACE}/revocation-results.json"
        --output-html "${WORKSPACE}/revocation-report.html"
    )
fi

set +e
go run . "${GO_RUN_ARGS[@]}"
EXIT_CODE=$?
set -e

# Status line — single source of truth is the exit code. The framework's own
# end-of-run summary is in the stdout above this point (captured in the Jenkins
# console log) for anyone who wants per-test detail.
echo
echo "============================================================"
if [ "$EXIT_CODE" -eq 0 ]; then
    echo "Revocation Validation: PASSED"
else
    echo "Revocation Validation: FAILED (exit $EXIT_CODE)"
    echo "See the framework's per-test output above this line for failure details."
    if [ "$LOG_LEVEL" != "debug" ]; then
        echo "For verbose logs and machine-readable JSON/HTML artifacts, re-run with"
        echo "  REVOCATION_LOG_LEVEL=debug"
    fi
fi
echo "============================================================"

# In debug mode, tell Jenkins viewers where the artifacts went.
if [ "$LOG_LEVEL" = "debug" ]; then
    if [ -f "${WORKSPACE}/revocation-results.json" ]; then
        chmod 0644 "${WORKSPACE}/revocation-results.json" 2>/dev/null || true
        echo "[Info] JSON results: ${WORKSPACE}/revocation-results.json ($(wc -c < "${WORKSPACE}/revocation-results.json" 2>/dev/null || echo '?') bytes)"
    fi
    if [ -f "${WORKSPACE}/revocation-report.html" ]; then
        chmod 0644 "${WORKSPACE}/revocation-report.html" 2>/dev/null || true
        echo "[Info] HTML report:  ${WORKSPACE}/revocation-report.html ($(wc -c < "${WORKSPACE}/revocation-report.html" 2>/dev/null || echo '?') bytes)"
    fi
    echo "[Info] Files are accessible via the Jenkins build's Workspace link."
fi

exit $EXIT_CODE
