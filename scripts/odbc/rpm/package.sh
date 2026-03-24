#!/bin/bash
#
# Package the Snowflake ODBC driver as an RPM.
#
# Assumes the driver has already been built (e.g. via the build script).
# Must be run from the repository root.
#
# Prerequisites:
#   - fpm (Ruby gem: gem install fpm)
#
# Environment variables:
#   PLATFORM - e.g. "linux-x86_64-glibc" or "linux-aarch64-glibc"
#
set -euxo pipefail

source ./scripts/version.sh

echo "=== Platform: $PLATFORM ==="

case "$PLATFORM" in
    linux-x86_64-glibc)
        PLATFORM_TARGET="x86_64-unknown-linux-gnu"
        SYSTEM_ARCH="x86_64"
        ;;
    linux-aarch64-glibc)
        PLATFORM_TARGET="aarch64-unknown-linux-gnu"
        SYSTEM_ARCH="aarch64"
        ;;
    *)
        echo "Unsupported platform for RPM: $PLATFORM"
        exit 1
        ;;
esac

if [[ "$(uname -m)" != "$SYSTEM_ARCH" ]]; then
    echo "Architecture mismatch: PLATFORM=$PLATFORM expects $SYSTEM_ARCH but running on $(uname -m)"
    exit 1
fi

DRIVER_SO="target/$PLATFORM_TARGET/release/libsfodbc.so"

if [[ ! -f "$DRIVER_SO" ]]; then
    echo "Driver not found at $DRIVER_SO. Build it first."
    exit 1
fi

BUILD_DIR=build
ODBC_DIR=/usr/lib64/snowflake/odbc
STAGE_DIR=$(mktemp -d)
trap 'rm -rf "$STAGE_DIR"' EXIT
RPM_SCRIPTS_DIR=scripts/odbc/rpm

echo "=== Staging files in $STAGE_DIR ==="
mkdir -p "$STAGE_DIR$ODBC_DIR/lib"
mkdir -p "$STAGE_DIR$ODBC_DIR/include"
cp "$DRIVER_SO" "$STAGE_DIR$ODBC_DIR/lib/"
cp odbc/include/sf_odbc.h "$STAGE_DIR$ODBC_DIR/include/"

RPM_NAME="snowflake-odbc-${VERSION}.${SYSTEM_ARCH}.rpm"
mkdir -p "$BUILD_DIR"

echo "=== Building RPM: $RPM_NAME ==="
fpm -s dir \
    -t rpm \
    -n snowflake-odbc \
    -v "$VERSION" \
    -C "$STAGE_DIR" \
    -p "$BUILD_DIR/$RPM_NAME" \
    -d unixODBC \
    --url https://www.snowflake.net/ \
    --description "Snowflake ODBC Driver ($VERSION, Release)" \
    --license "Commercial" \
    --vendor "Snowflake Computing, Inc." \
    --rpm-changelog "$RPM_SCRIPTS_DIR/changelog" \
    --after-install "$RPM_SCRIPTS_DIR/after_install.sh" \
    --before-remove "$RPM_SCRIPTS_DIR/before_remove.sh" \
    "${ODBC_DIR:1}"

rm -rf "$STAGE_DIR"

echo "=== Successfully created RPM at $BUILD_DIR/$RPM_NAME ==="
