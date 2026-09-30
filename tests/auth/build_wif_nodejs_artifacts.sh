#!/bin/bash -e
#
# Builds a Node.js tree that can run WIF e2e tests in a Node Docker image.
# Output: tests/auth/wif/artifacts/nodejs_wif.tar.gz

set -o pipefail

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( cd "$THIS_DIR/../.." && pwd )"
ARTIFACT_DIR="$THIS_DIR/wif/artifacts"
ARTIFACT="$ARTIFACT_DIR/nodejs_wif.tar.gz"

mkdir -p "$ARTIFACT_DIR"
cd "$REPO_ROOT/nodejs"

npm install
npm run build:core
npm run build:sdk
npm run build:link-local

tar -czf "$ARTIFACT" -C "$REPO_ROOT" \
  nodejs/package.json \
  nodejs/tsconfig.json \
  nodejs/vitest.config.ts \
  nodejs/tests \
  nodejs/_build \
  nodejs/node_modules

echo "Staged $ARTIFACT"
ls -la "$ARTIFACT"
