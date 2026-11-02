#!/bin/bash
#
# Pre-remove script for the Snowflake ODBC driver RPM and deb.
# Runs on the customer's machine before the package is removed.
#
# Unregisters the driver and DSN from unixODBC on a full removal.
# An upgrade leaves both in place: the new package's post-install keeps a
# recorded name at the same UsageCount and unregisters a recorded name it
# replaces. rpm runs that post-install before this script, so unregistering
# here would remove the registration the upgrade keeps.
#

ODBC_DIR=/usr/lib64/snowflake/odbc
# shellcheck source=driver_name.sh
source "$ODBC_DIR/driver_name.sh"

if full_removal "${1:-}"; then
    DRIVER_NAME="$(removal_driver_name)"
    odbcinst -u -d -n "$DRIVER_NAME" || true
    delete_recorded_driver_name

    SYSTEM_DSN_PATH=$(odbcinst -j | grep "SYSTEM DATA SOURCES" | sed -n -e 's/SYSTEM DATA SOURCES: //p')
    OLD_ODBC_INI=${ODBCINI-}
    export ODBCINI="$SYSTEM_DSN_PATH"
    odbcinst -u -s -l -n snowflake || true
    if [[ -z "$OLD_ODBC_INI" ]]; then
        unset ODBCINI
    else
        export ODBCINI="$OLD_ODBC_INI"
    fi
fi
