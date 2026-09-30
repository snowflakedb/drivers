#!/bin/bash -e
#
# Builds a Node.js tree that can run WIF e2e tests in a Node Docker image.
# Output: tests/auth/wif/artifacts/nodejs_wif.tar.gz

set -o pipefail

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$THIS_DIR/../.." && pwd )"
ARTIFACT_DIR="$THIS_DIR/wif/artifacts"
ARTIFACT="$ARTIFACT_DIR/nodejs_wif.tar.gz"

log() {
  printf '[wif-artifact][%s] %s\n' "$(date -u +'%H:%M:%S')" "$*"
}

mkdir -p "$ARTIFACT_DIR"
cd "$REPO_ROOT/nodejs"

log "npm install"
npm install
log "Building the native core (cargo, several minutes cold)"
npm run build:core
log "Building the SDK"
npm run build:sdk
npm run build:link-local

# node_modules dominates this tarball and it is shipped to every provider VM,
# so its size is worth seeing next to the transfer times in the lane log.
log "Packing $(du -sh node_modules | cut -f1) of node_modules into the artifact"
pack_started=$SECONDS
tar -czf "$ARTIFACT" -C "$REPO_ROOT" \
  nodejs/package.json \
  nodejs/tsconfig.json \
  nodejs/vitest.config.ts \
  nodejs/tests \
  nodejs/_build \
  nodejs/node_modules
log "Packed in $((SECONDS - pack_started))s"

echo "Staged $ARTIFACT"
ls -la "$ARTIFACT"
