#!/bin/bash
#
# Package the Snowflake ODBC UD driver as a macOS DMG (universal binary).
#
# Builds or accepts pre-built dylibs for x86_64 and aarch64, merges them
# with lipo into a universal binary, creates a .pkg installer, and wraps
# it in a .dmg.
#
# Must be run from the repository root on macOS.
#
# Environment variables (optional):
#   DRIVER_X86_64  - path to x86_64 libsfodbc.dylib (default: target/x86_64-apple-darwin/release/libsfodbc.dylib)
#   DRIVER_ARM64   - path to aarch64 libsfodbc.dylib (default: target/aarch64-apple-darwin/release/libsfodbc.dylib)
#
set -euxo pipefail

source ./odbc/version.sh

INSTALL_DIR="/opt/snowflake/snowflakeodbcud"
PKG_IDENTIFIER="net.snowflake.odbc-ud"
BUILD_DIR=build
SCRIPTS_DIR=odbc/installer/mac/scripts

DRIVER_X86_64="${DRIVER_X86_64:-target/x86_64-apple-darwin/release/libsfodbc.dylib}"
DRIVER_ARM64="${DRIVER_ARM64:-target/aarch64-apple-darwin/release/libsfodbc.dylib}"

for f in "$DRIVER_X86_64" "$DRIVER_ARM64"; do
    if [[ ! -f "$f" ]]; then
        echo "Driver not found at $f. Build it first."
        exit 1
    fi
done

STAGE_DIR=$(mktemp -d)
trap 'rm -rf "$STAGE_DIR"' EXIT

echo "=== Creating universal binary ==="
mkdir -p "$STAGE_DIR$INSTALL_DIR/lib"
lipo -create "$DRIVER_X86_64" "$DRIVER_ARM64" -output "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"
lipo -info "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"

echo "=== Staging additional files ==="
mkdir -p "$STAGE_DIR$INSTALL_DIR/include"
cp odbc/include/sf_odbc.h "$STAGE_DIR$INSTALL_DIR/include/"

mkdir -p "$BUILD_DIR"

PKG_NAME="snowflake_odbc_ud-${VERSION}-universal.pkg"
DMG_NAME="snowflake_odbc_ud-${VERSION}-universal.dmg"

echo "=== Building pkg: $PKG_NAME ==="
pkgbuild \
    --identifier "$PKG_IDENTIFIER" \
    --version "$VERSION" \
    --install-location "$INSTALL_DIR" \
    --root "$STAGE_DIR$INSTALL_DIR" \
    --scripts "$SCRIPTS_DIR" \
    "$BUILD_DIR/$PKG_NAME"

echo "=== Creating DMG: $DMG_NAME ==="
hdiutil create \
    -volname "Snowflake ODBC UD" \
    -srcfolder "$BUILD_DIR/$PKG_NAME" \
    -ov \
    -format UDZO \
    "$BUILD_DIR/$DMG_NAME"

rm -rf "$STAGE_DIR"

echo "=== Successfully created DMG at $BUILD_DIR/$DMG_NAME ==="
