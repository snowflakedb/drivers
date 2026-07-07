#!/bin/bash -e
#
# Builds the WIF e2e test artifacts on the Jenkins node, inside the
# rhel8-universal-driver-coverage image (which carries the Rust toolchain and
# the C/C++ build deps). The resulting binaries are staged under
# ci/wif/artifacts/ so the outer ci/test_wif.sh can scp them to the bare WIF
# cloud VMs and run them there in a public runtime container.
#
# Why prebuild here instead of on the VM: the WIF VMs have Docker + scp but no
# Rust/cargo, no cmake/gcc, no rsync, and no access to our private Artifactory
# (so they cannot pull the coverage image). They CAN pull public DockerHub
# images. This mirrors how snowflake-odbc tests WIF: build artifacts on the
# Jenkins node, ship them, run them in a public container on the VM.
set -o pipefail

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$THIS_DIR/.." && pwd )"
ARTIFACT_DIR="$THIS_DIR/wif/artifacts"

cd "$REPO_ROOT"
rm -rf "$ARTIFACT_DIR"
mkdir -p "$ARTIFACT_DIR"

# ---------------------------------------------------------------------------
# sf_core WIF e2e test binary
# ---------------------------------------------------------------------------
# Build (not run) the e2e_tests target. We vendor OpenSSL so the shipped binary
# depends only on glibc/libgcc at runtime, keeping the public runtime container
# minimal and avoiding an openssl-libs version mismatch on the VM.
echo "Building sf_core e2e test binary (target: e2e_tests)..."
SF_CORE_BIN="$(
  cargo test -p sf_core --test e2e_tests --features vendored-openssl --no-run \
    --message-format=json \
    | jq -r 'select(.reason=="compiler-artifact" and .target.name=="e2e_tests" and .executable != null) | .executable' \
    | tail -n 1
)"

if [[ -z "$SF_CORE_BIN" || ! -f "$SF_CORE_BIN" ]]; then
  echo "ERROR: could not locate the built e2e_tests binary" >&2
  exit 1
fi

echo "Built sf_core e2e binary: $SF_CORE_BIN"
cp "$SF_CORE_BIN" "$ARTIFACT_DIR/sf_core_e2e"
chmod +x "$ARTIFACT_DIR/sf_core_e2e"

echo "WIF artifacts staged in $ARTIFACT_DIR:"
ls -la "$ARTIFACT_DIR"
