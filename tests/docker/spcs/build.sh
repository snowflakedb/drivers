#!/bin/bash
#
# Build the SPCS auth e2e probe image (linux/amd64).
#
# The build context is the repository root (the image compiles sf_core), so this
# script resolves the repo root regardless of where it is invoked from.
#
# Usage:
#   ./tests/docker/spcs/build.sh
#
# Optional environment variables:
#   PLATFORM   - docker build platform (default: linux/arm64; the SPCS pool uses
#                an ARM instance family, GEN_ARM_G1_2)
#   IMAGE_TAG  - full local tag override (default: <image-name>:<version>)
#
# To push to a Snowflake image repository, log in and retag/push:
#   snow spcs image-registry login
#   docker tag <local-tag> <org>-<account>.registry.snowflakecomputing.com/testing_setup/public/ud_test_image_repo/spcs_probe:<version>
#   docker push <org>-<account>.registry.snowflakecomputing.com/testing_setup/public/ud_test_image_repo/spcs_probe:<version>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

PLATFORM="${PLATFORM:-linux/arm64}"
IMAGE_NAME="snowflakedb/snowdrivers-test-spcs-universal-driver"
IMAGE_VERSION="1"
LOCAL_TAG="${IMAGE_TAG:-${IMAGE_NAME}:${IMAGE_VERSION}}"

docker build \
    --platform="$PLATFORM" \
    --file "$SCRIPT_DIR/Dockerfile" \
    --tag "$LOCAL_TAG" \
    "$REPO_ROOT"

echo "Built: ${LOCAL_TAG}"
echo ""
echo "To push to a Snowflake image repository:"
echo "  snow spcs image-registry login"
echo "  docker tag ${LOCAL_TAG} <registry>/testing_setup/public/ud_test_image_repo/spcs_probe:${IMAGE_VERSION}"
echo "  docker push <registry>/testing_setup/public/ud_test_image_repo/spcs_probe:${IMAGE_VERSION}"
