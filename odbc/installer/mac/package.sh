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
#   PKG_VARIANT    - empty for the standard driver, or "fips" for the FIPS build.
#                    The FIPS package is named snowflake-odbc-fips and ships the
#                    AWS-LC crypto library next to libsfodbc.dylib.
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

PKG_VARIANT="${PKG_VARIANT:-}"
case "$PKG_VARIANT" in
    ""|fips) ;;
    *)
        echo "Unsupported PKG_VARIANT: $PKG_VARIANT"
        exit 1
        ;;
esac

stage_fips_slice() {
    local driver="$1"
    local slice_dir="$2"
    local dep base src found_crypto
    mkdir -p "$slice_dir"
    cp "$driver" "$slice_dir/libsfodbc.dylib"
    found_crypto=0
    while read -r dep; do
        [[ -z "$dep" ]] && continue
        base=$(basename "$dep")
        if [[ -f "$dep" ]]; then
            src="$dep"
        else
            src=$(find "$(dirname "$driver")" -name "$base" -type f -print -quit)
        fi
        if [[ -z "${src}" || ! -f "$src" ]]; then
            echo "FIPS library $base not found for $driver"
            exit 1
        fi
        cp "$src" "$slice_dir/$base"
        install_name_tool -id "@loader_path/$base" "$slice_dir/$base"
        install_name_tool -change "$dep" "@loader_path/$base" "$slice_dir/libsfodbc.dylib"
        case "$base" in
            libaws_lc_fips_*_crypto.dylib) found_crypto=1 ;;
        esac
    done < <(otool -L "$slice_dir/libsfodbc.dylib" | awk '/libaws_lc_fips_.*\.dylib/ { print $1 }')
    if [[ "$found_crypto" -ne 1 ]]; then
        echo "Driver $driver does not link an AWS-LC FIPS crypto library"
        exit 1
    fi
}

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
EXPAND_PARENT=$(mktemp -d)
trap 'rm -rf "$STAGE_DIR" "$SCRIPTS_STAGE_DIR" "$EXPAND_PARENT"' EXIT

echo "=== Creating universal binary ==="
mkdir -p "$STAGE_DIR$INSTALL_DIR/lib"
if [[ "$PKG_VARIANT" == "fips" ]]; then
    SLICE_DIR=$(mktemp -d)
    trap 'rm -rf "$STAGE_DIR" "$SCRIPTS_STAGE_DIR" "$SLICE_DIR"' EXIT
    stage_fips_slice "$DRIVER_X86_64" "$SLICE_DIR/x86_64"
    stage_fips_slice "$DRIVER_ARM64" "$SLICE_DIR/arm64"
    lipo -create \
        "$SLICE_DIR/x86_64/libsfodbc.dylib" \
        "$SLICE_DIR/arm64/libsfodbc.dylib" \
        -output "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"
    shopt -s nullglob
    fips_libs=("$SLICE_DIR/x86_64"/libaws_lc_fips_*.dylib)
    shopt -u nullglob
    if [[ ${#fips_libs[@]} -eq 0 ]]; then
        echo "No AWS-LC FIPS libraries were staged"
        exit 1
    fi
    for crypto in "${fips_libs[@]}"; do
        name=$(basename "$crypto")
        if [[ ! -f "$SLICE_DIR/arm64/$name" ]]; then
            echo "FIPS library $name is missing from the arm64 slice"
            exit 1
        fi
        lipo -create "$crypto" "$SLICE_DIR/arm64/$name" \
            -output "$STAGE_DIR$INSTALL_DIR/lib/$name"
    done
else
    lipo -create "$DRIVER_X86_64" "$DRIVER_ARM64" -output "$STAGE_DIR$INSTALL_DIR/lib/libsfodbc.dylib"
fi
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

if [[ "$PKG_VARIANT" == "fips" ]]; then
    PKG_NAME="snowflake-odbc-fips-${VERSION}.universal.pkg"
else
    PKG_NAME="snowflake-odbc-${VERSION}.universal.pkg"
fi

echo "=== Building pkg: $PKG_NAME ==="
pkgbuild \
    --identifier "$PKG_IDENTIFIER" \
    --version "$VERSION" \
    --install-location "$INSTALL_DIR" \
    --root "$STAGE_DIR$INSTALL_DIR" \
    --scripts "$SCRIPTS_STAGE_DIR" \
    --ownership recommended \
    "$BUILD_DIR/$PKG_NAME"

pkgutil --expand "$BUILD_DIR/$PKG_NAME" "$EXPAND_PARENT/pkg"
if lsbom -p u "$EXPAND_PARENT/pkg/Bom" | grep -qvx '0'; then
    echo "Payload BOM records a non-root uid:"
    lsbom -p Mugf "$EXPAND_PARENT/pkg/Bom"
    exit 1
fi

echo "=== Successfully created pkg at $BUILD_DIR/$PKG_NAME ==="
