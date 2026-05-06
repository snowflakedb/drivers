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

read_odbc_metadata() {
    local key="$1"
    awk -v key="$key" '
        /^\[package\.metadata\.odbc\][[:space:]]*$/ { in_section = 1; next }
        /^\[/                                       { in_section = 0 }
        in_section && $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
            line = $0
            sub(/^[^"]*"/, "", line)
            sub(/".*$/,    "", line)
            print line
            exit
        }
    ' odbc/Cargo.toml
}

BASE_VERSION=$(read_odbc_metadata odbc_preview_version)
ODBC_API_VERSION=$(read_odbc_metadata odbc_api_version)
if [[ -z "$BASE_VERSION" || -z "$ODBC_API_VERSION" ]]; then
    echo "Failed to read odbc_preview_version / odbc_api_version from [package.metadata.odbc] in odbc/Cargo.toml"
    exit 1
fi
COMMIT_HASH="${COMMIT_HASH:-$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")}"
VERSION="${BASE_VERSION}-${COMMIT_HASH}"

INSTALL_DIR="/opt/snowflake/snowflakeodbcud"
PKG_IDENTIFIER="net.snowflake.odbc-ud"
BUILD_DIR=build
SCRIPTS_DIR=odbc/installer/mac/scripts
TEMPLATES_DIR=odbc/installer/shared/templates

DRIVER_X86_64="${DRIVER_X86_64:-target/x86_64-apple-darwin/release/libsfodbc.dylib}"
DRIVER_ARM64="${DRIVER_ARM64:-target/aarch64-apple-darwin/release/libsfodbc.dylib}"

for f in "$DRIVER_X86_64" "$DRIVER_ARM64"; do
    if [[ ! -f "$f" ]]; then
        echo "Driver not found at $f. Build it first."
        exit 1
    fi
done

STAGE_DIR=$(mktemp -d)
SCRIPTS_STAGE_DIR=$(mktemp -d)
trap 'rm -rf "$STAGE_DIR" "$SCRIPTS_STAGE_DIR"' EXIT

echo "=== Creating universal binary ==="
mkdir -p "$STAGE_DIR$INSTALL_DIR/lib"
lipo -create "$DRIVER_X86_64" "$DRIVER_ARM64" -output "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"
lipo -info "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"

echo "=== Staging additional files ==="
mkdir -p "$STAGE_DIR$INSTALL_DIR/include"
cp odbc/include/sf_odbc.h "$STAGE_DIR$INSTALL_DIR/include/"

# pkgbuild --scripts packages everything in the directory it points at, so
# stage the postinstall alongside the iODBC ini templates the postinstall
# renders at install time. __ODBC_API_VERSION__ is baked into the odbcinst
# template here so the customer's machine doesn't need access to Cargo.toml;
# the remaining placeholders are resolved by the postinstall.
cp "$SCRIPTS_DIR/postinstall" "$SCRIPTS_STAGE_DIR/postinstall"
chmod +x "$SCRIPTS_STAGE_DIR/postinstall"
sed "s/__ODBC_API_VERSION__/${ODBC_API_VERSION}/g" \
    "$TEMPLATES_DIR/odbcinst.ini.template" > "$SCRIPTS_STAGE_DIR/odbcinst.ini.template"
cp "$TEMPLATES_DIR/odbc.ini.template" "$SCRIPTS_STAGE_DIR/odbc.ini.template"

mkdir -p "$BUILD_DIR"

PKG_NAME="snowflake-odbc-ud-${VERSION}-universal.pkg"
DMG_NAME="snowflake-odbc-ud-${VERSION}-universal.dmg"

echo "=== Building pkg: $PKG_NAME ==="
pkgbuild \
    --identifier "$PKG_IDENTIFIER" \
    --version "$VERSION" \
    --install-location "$INSTALL_DIR" \
    --root "$STAGE_DIR$INSTALL_DIR" \
    --scripts "$SCRIPTS_STAGE_DIR" \
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
