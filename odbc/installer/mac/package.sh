#!/bin/bash
#
# Package the Snowflake ODBC Driver as a macOS .pkg (universal binary).
#
# Builds or accepts pre-built dylibs for x86_64 and aarch64, merges them
# with lipo into a universal binary and produces a flat .pkg installer
# via pkgbuild.
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

read_package_version() {
    awk '
        /^\[package\][[:space:]]*$/ { in_section = 1; next }
        /^\[/                       { in_section = 0 }
        in_section && /^[[:space:]]*version[[:space:]]*=/ {
            line = $0
            sub(/^[^"]*"/, "", line)
            sub(/".*$/,    "", line)
            print line
            exit
        }
    ' odbc/Cargo.toml
}

BASE_VERSION=$(read_package_version)
ODBC_API_VERSION=$(read_odbc_metadata odbc_api_version)
if [[ -z "$BASE_VERSION" || -z "$ODBC_API_VERSION" ]]; then
    echo "Failed to read package version / odbc_api_version from odbc/Cargo.toml"
    exit 1
fi
COMMIT_HASH="${COMMIT_HASH:-$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")}"
VERSION="${BASE_VERSION}"

INSTALL_DIR="/opt/snowflake/snowflakeodbc"
PKG_IDENTIFIER="net.snowflake.odbc"
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

# Ship a default sf.odbc.ini next to the dylib
cp odbc/installer/mac/sf.odbc.ini "$STAGE_DIR$INSTALL_DIR/sf.odbc.ini"

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

PKG_NAME="snowflake-odbc-${VERSION}-universal.pkg"

echo "=== Building pkg: $PKG_NAME ==="
pkgbuild \
    --identifier "$PKG_IDENTIFIER" \
    --version "$VERSION" \
    --install-location "$INSTALL_DIR" \
    --root "$STAGE_DIR$INSTALL_DIR" \
    --scripts "$SCRIPTS_STAGE_DIR" \
    "$BUILD_DIR/$PKG_NAME"

echo "=== Successfully created pkg at $BUILD_DIR/$PKG_NAME ==="
