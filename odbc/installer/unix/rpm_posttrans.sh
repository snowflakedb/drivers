#!/bin/bash
#
# Post-transaction script for the Snowflake ODBC driver RPM.
# rpm runs it after the whole install or upgrade, including the old
# package's pre-remove.
#
# A package built before the driver-name record unregisters
# ${SF_DRIVER_NAME:-Snowflake ODBC} and the snowflake DSN in its pre-remove
# even during an upgrade. That can delete the driver and DSN this package's
# post-install registered, so they are registered again here when missing.
#

ODBC_DIR=/usr/lib64/snowflake/odbc
# shellcheck source=driver_name.sh
source "$ODBC_DIR/driver_name.sh"
DRIVER_NAME="$(read_recorded_driver_name)" || DRIVER_NAME="$(install_driver_name)"

if ! driver_registered "$DRIVER_NAME"; then
    echo "Restoring driver info in odbcinst.ini..."
    register_driver "$DRIVER_NAME"
fi

if ! dsn_registered; then
    echo "Restoring connect info in odbc.ini..."
    register_dsn "$DRIVER_NAME" "$(dsn_account)"
fi
