#!/bin/bash
#
# Post-install script for the Snowflake ODBC driver RPM.
# Runs on the customer's machine after "rpm -i".
#
# Registers the driver with unixODBC and creates a template DSN.
#
# Driver registration and DSN content live in ini section templates installed
# alongside the driver. odbc/installer/unix/package.sh substitutes
# __ODBC_API_VERSION__ at packaging time from [package.metadata.odbc] in
# odbc/Cargo.toml; the remaining placeholders are substituted here at install time.
#

ODBC_DIR=/usr/lib64/snowflake/odbc
DRIVER_PATH=$ODBC_DIR/lib/libsfodbc.so
TEMPLATES_DIR=$ODBC_DIR/templates

if [[ -z "$SF_ACCOUNT" ]]; then
    echo "[WARN] SF_ACCOUNT is not set, please manually update the odbc.ini file after installation"
    SF_ACCOUNT=SF_ACCOUNT
fi

render_template() {
    sed \
        -e "s|__DRIVER_PATH__|${DRIVER_PATH}|g" \
        -e "s|__SF_ACCOUNT__|${SF_ACCOUNT}|g" \
        "$1"
}

echo "Adding driver info to odbcinst.ini..."
render_template "$TEMPLATES_DIR/odbcinst.ini.template" | odbcinst -i -d -r

echo "Adding connect info to odbc.ini..."
render_template "$TEMPLATES_DIR/odbc.ini.template" | odbcinst -i -s -l -r
