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
# Also clean up the shim directory on exit (extend the existing cleanup trap).
trap 'cleanup; rm -rf "$CARGO_SHIM_DIR"' EXIT
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

# Output policy: NO reports. The framework writes structured JSON to a tmp file
# only so we can parse it for a console-log summary at the end; the file itself
# is deleted before exit and never archived. The HTML report is not generated at
# all (no --output-html flag) — even on failure, the Jenkins console log carries
# the framework's per-test stdout, which is sufficient for triage. This trims the
# stage's per-build cost on Jenkins from "archive + publish multi-MB artifacts" to
# "log a few lines".
#
# Log level: `info` keeps stdout compact. Override with REVOCATION_LOG_LEVEL=debug
# (env var) for verbose investigation when triaging a specific failure.
LOG_LEVEL="${REVOCATION_LOG_LEVEL:-info}"
RESULTS_JSON="$(mktemp "${TMPDIR:-/tmp}/revocation-results.XXXXXX.json")"

# Extend the EXIT trap so the tmp results file is always cleaned up, even on errors.
trap 'cleanup; rm -rf "$CARGO_SHIM_DIR"; rm -f "$RESULTS_JSON"' EXIT

set +e
go run . \
    --client universal-driver-rust \
    --universal-driver-path "$DRIVER_ROOT" \
    --output "$RESULTS_JSON" \
    --log-level "$LOG_LEVEL"
EXIT_CODE=$?
set -e

# Print a compact one-screen summary derived from the JSON. Falls back gracefully
# if jq/python aren't available or the JSON is malformed — we still get the exit
# code as the source of truth, the summary is just a nicety for the console reader.
echo
echo "============================================================"
echo "Revocation Validation Summary"
echo "============================================================"
if [ -s "$RESULTS_JSON" ]; then
    if command -v jq >/dev/null 2>&1; then
        # The framework's JSON shape: { "total": N, "passed": N, "failed": N, "skipped": N, "tests": [...] }
        # `// "?"` defaults missing keys to "?" so we never fail on schema drift.
        jq -r '
            "  Total:    \(.total // "?")\n" +
            "  Passed:   \(.passed // "?")\n" +
            "  Failed:   \(.failed // "?")\n" +
            "  Skipped:  \(.skipped // "?")"
        ' "$RESULTS_JSON" 2>/dev/null || echo "  (jq parse failed — see exit code below)"
        # On failure, also list the failing test names (one per line, truncated to 20).
        if [ "$EXIT_CODE" -ne 0 ]; then
            echo
            echo "  Failed tests:"
            jq -r '.tests[]? | select(.status == "failed" or .status == "FAIL" or .passed == false) | "    - \(.name // .id // "<unnamed>")"' \
                "$RESULTS_JSON" 2>/dev/null | head -n 20 || true
        fi
    else
        echo "  (jq not available — summary skipped; see framework stdout above for per-test results)"
    fi
else
    echo "  (no results file produced — see framework stdout above)"
fi
echo "============================================================"

if [ "$EXIT_CODE" -eq 0 ]; then
    echo "[Info] Revocation Validation passed."
else
    echo "[Warn] Revocation Validation FAILED (exit $EXIT_CODE) — see Jenkins console log above for per-test details."
fi

exit $EXIT_CODE
