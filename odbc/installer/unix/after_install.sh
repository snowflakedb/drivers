#!/bin/bash
#
# Post-install script for the Snowflake ODBC driver RPM and deb.
# Runs on the customer's machine after the package is installed.
#
# Registers the driver with unixODBC and creates a template DSN.
#
# Driver registration and DSN content live in ini section templates installed
# alongside the driver. odbc/installer/unix/package.sh substitutes
# __ODBC_API_VERSION__ at packaging time from [package.metadata.odbc] in
# odbc/Cargo.toml; driver_name.sh substitutes the remaining placeholders at
# install time.
#

ODBC_DIR=/usr/lib64/snowflake/odbc
# shellcheck source=driver_name.sh
source "$ODBC_DIR/driver_name.sh"
DRIVER_NAME="$(install_driver_name)"
SF_ACCOUNT="$(dsn_account)"

# A record means the installed package registered that name once, and its
# pre-remove leaves the registration in place on upgrade. A package without a
# record unregisters ${SF_DRIVER_NAME:-Snowflake ODBC} and the snowflake DSN in
# its pre-remove on upgrade too. deb runs that pre-remove before this script;
# rpm runs it after, and rpm_posttrans.sh restores whatever it removed.
echo "Adding driver info to odbcinst.ini..."
previous_name="$(read_recorded_driver_name)" || previous_name=""
if [[ "$previous_name" == "$DRIVER_NAME" ]]; then
    refresh_driver "$DRIVER_NAME"
else
    if [[ -n "$previous_name" ]]; then
        odbcinst -u -d -n "$previous_name" || true
    fi
    register_driver "$DRIVER_NAME"
fi
write_recorded_driver_name "$DRIVER_NAME"

echo "Adding connect info to odbc.ini..."
register_dsn "$DRIVER_NAME" "$SF_ACCOUNT"
