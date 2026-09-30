#!/bin/bash
#
# Builds the Node.js WIF artifact inside the linux/amd64 container that
# tests/auth/run_wif_local.sh starts. Reads the repo from /source (read-only)
# and leaves tests/auth/wif/artifacts/nodejs_wif.tar.gz in /artifacts.
# Apt, npm 11, and rustup come from tests/docker/wif-nodejs/Dockerfile.

set -euo pipefail

WORKSPACE=/workspace

# /source is read-only. Node and Rust need a writable tree, so this copies the
# mount into $WORKSPACE. `nodejs/_build` and `nodejs/node_modules` are left
# behind on purpose: on a macOS host they hold Darwin binaries, and napi would
# load the host `.node` instead of building the linux one.
mkdir -p "$WORKSPACE"
tar -C /source \
  --exclude=.git \
  --exclude=target \
  --exclude=nodejs/node_modules \
  --exclude=nodejs/_build \
  --exclude=tests/auth/wif/artifacts \
  -cf - . | tar -C "$WORKSPACE" -xf -

export PATH="/root/.cargo/bin:$PATH"
cd "$WORKSPACE"
# No toolchain version here: rustup installs the channel rust-toolchain.toml
# names, the same way the perf-test images do.
rustup show

export CARGO_TARGET_DIR=/cargo-target
./tests/auth/build_wif_nodejs_artifacts.sh
cp "$WORKSPACE/tests/auth/wif/artifacts/nodejs_wif.tar.gz" /artifacts/nodejs_wif.tar.gz
