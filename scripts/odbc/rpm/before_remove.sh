#!/bin/bash
#
# Pre-remove script for the Snowflake ODBC driver RPM.
# Runs on the customer's machine before "rpm -e".
#
# Unregisters the driver and DSN from unixODBC.
#

odbcinst -u -d -n SnowflakeDSIIDriver || true

SYSTEM_DSN_PATH=$(odbcinst -j | grep "SYSTEM DATA SOURCES" | sed -n -e 's/SYSTEM DATA SOURCES: //p')
OLD_ODBC_INI=${ODBCINI-}
export ODBCINI="$SYSTEM_DSN_PATH"
odbcinst -u -s -l -n snowflake || true
if [[ -z "$OLD_ODBC_INI" ]]; then
    unset ODBCINI
else
    export ODBCINI="$OLD_ODBC_INI"
fi
