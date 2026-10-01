#!/bin/bash
#
# Builds the sf_core WIF test binary inside the linux/amd64 container that
# tests/auth/run_wif_local.sh sf_core starts. Reads the repo from /source
# (read-only) and leaves sf_core_e2e in /artifacts. The image shares the
# rockylinux:8 base that tests/auth/run_wif.sh runs the binary in.

set -euo pipefail

WORKSPACE=/workspace

log() {
  printf '[wif-build][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

log "Copying the repository into $WORKSPACE"
mkdir -p "$WORKSPACE"
tar -C /source \
  --exclude=.git \
  --exclude=target \
  --exclude=nodejs/node_modules \
  --exclude=nodejs/_build \
  --exclude=tests/auth/wif/artifacts \
  -cf - . | tar -C "$WORKSPACE" -xf -

cd "$WORKSPACE"
rustup show

export CARGO_TARGET_DIR=/cargo-target
./tests/auth/build_wif_sf_core_artifacts.sh
cp "$WORKSPACE/tests/auth/wif/artifacts/sf_core_e2e" /artifacts/sf_core_e2e
log "Artifact ready: $(du -h /artifacts/sf_core_e2e | cut -f1)"
