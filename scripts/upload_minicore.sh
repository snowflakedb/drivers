#!/bin/bash
# Exit on any error
set -euxo pipefail

source ./scripts/version.sh

echo "=== Platform: $PLATFORM ==="
echo "=== Commit SHA: $COMMIT_SHA ==="

# Set build directory
BUILD_DIR=$(pwd)/build

PACKAGE_NAME=sf_mini_core_${PLATFORM}_${VERSION}_SNAPSHOT_${COMMIT_SHA}.tar.gz
UPLOAD_PATH=s3://sfc-eng-jenkins/universal-driver/sf_mini_core/$PACKAGE_NAME

echo "=== Uploading archive to S3: $UPLOAD_PATH ==="

aws s3 cp $BUILD_DIR/$PACKAGE_NAME $UPLOAD_PATH

echo "=== Successfully uploaded archive to S3: $UPLOAD_PATH ==="

