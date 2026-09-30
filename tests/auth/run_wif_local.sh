#!/bin/bash
#
# Builds the linux/amd64 Node WIF artifact in Docker, then runs the same
# tests/auth/run_wif.sh Jenkins runs, against the same three WIF VMs.
#   ./scripts/decode_secrets.sh wif
#   ./tests/auth/run_wif_local.sh
#
# This helper builds the Node artifact and then execs
# `tests/auth/run_wif.sh nodejs`. It does not select sf_core.
#
# Decode WIF secrets first. Do not use parameters_preprod.json.
#
# The builder image under tests/docker/wif-nodejs caches apt, npm 11, and
# rustup. Named volumes keep cargo, npm, rustup, and the Cargo target dir
# across runs.

set -euo pipefail

log() {
  printf '[wif-local][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

THIS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$THIS_DIR/../.." && pwd)"
NODEJS_BUILD_IMAGE="${NODEJS_BUILD_IMAGE:-ud-wif-nodejs-local:latest}"
PLATFORM=linux/amd64

if [[ $# -ne 0 ]]; then
  echo "ERROR: tests/auth/run_wif_local.sh takes no arguments" >&2
  exit 1
fi

for f in \
  "$THIS_DIR/wif/parameters/parameters_wif.json" \
  "$THIS_DIR/wif/parameters/rsa_wif_aws_azure" \
  "$THIS_DIR/wif/parameters/rsa_wif_gcp"
do
  if [[ ! -s "$f" ]]; then
    echo "ERROR: $f not found or empty" >&2
    echo "Run: ./scripts/decode_secrets.sh wif" >&2
    exit 1
  fi
done

mkdir -p "$THIS_DIR/wif/artifacts"

log "Building $NODEJS_BUILD_IMAGE"
PLATFORM="$PLATFORM" IMAGE_TAG="$NODEJS_BUILD_IMAGE" \
  "$REPO_ROOT/tests/docker/wif-nodejs/build.sh"

log "Building the linux/amd64 artifact in $NODEJS_BUILD_IMAGE"
docker run \
  --rm \
  --platform "$PLATFORM" \
  --user root \
  --volume "$REPO_ROOT:/source:ro" \
  --volume "$THIS_DIR/wif/artifacts:/artifacts" \
  --volume universal-driver-wif-cargo-registry:/root/.cargo/registry \
  --volume universal-driver-wif-cargo-git:/root/.cargo/git \
  --volume universal-driver-wif-npm-cache:/root/.npm \
  --volume universal-driver-wif-rustup:/root/.rustup \
  --volume universal-driver-wif-cargo-target:/cargo-target \
  "$NODEJS_BUILD_IMAGE" \
  bash /source/tests/docker/wif-nodejs/build_in_container.sh

log "Handing over to run_wif.sh nodejs"
exec "$THIS_DIR/run_wif.sh" nodejs
