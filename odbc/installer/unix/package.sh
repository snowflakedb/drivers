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
RPM_SCRIPTS_DIR=odbc/installer/unix
TEMPLATES_DIR=odbc/installer/shared/templates

echo "=== Staging files in $STAGE_DIR ==="
mkdir -p "$STAGE_DIR$ODBC_DIR/lib"
mkdir -p "$STAGE_DIR$ODBC_DIR/include"
mkdir -p "$STAGE_DIR$ODBC_DIR/templates"
cp "$DRIVER_SO" "$STAGE_DIR$ODBC_DIR/lib/"
cp odbc/include/sf_odbc.h "$STAGE_DIR$ODBC_DIR/include/"

sed "s/__ODBC_API_VERSION__/${ODBC_API_VERSION}/g" \
    "$TEMPLATES_DIR/odbcinst.ini.template" > "$STAGE_DIR$ODBC_DIR/templates/odbcinst.ini.template"
cp "$TEMPLATES_DIR/odbc.ini.template" "$STAGE_DIR$ODBC_DIR/templates/odbc.ini.template"

RPM_NAME="snowflake-odbc-ud-${VERSION}.${SYSTEM_ARCH}.rpm"
mkdir -p "$BUILD_DIR"

echo "=== Building RPM: $RPM_NAME ==="
fpm -s dir \
    -t rpm \
    -n snowflake-odbc-ud \
    -v "$BASE_VERSION" \
    -C "$STAGE_DIR" \
    -p "$BUILD_DIR/$RPM_NAME" \
    -d unixODBC \
    --url https://www.snowflake.net/ \
    --description "Snowflake ODBC UD ($VERSION, Release)" \
    --license "Commercial" \
    --vendor "Snowflake Computing, Inc." \
    --rpm-changelog "$RPM_SCRIPTS_DIR/changelog" \
    --after-install "$RPM_SCRIPTS_DIR/after_install.sh" \
    --before-remove "$RPM_SCRIPTS_DIR/before_remove.sh" \
    "${ODBC_DIR:1}"

rm -rf "$STAGE_DIR"

echo "=== Successfully created RPM at $BUILD_DIR/$RPM_NAME ==="
