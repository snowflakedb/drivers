#!/bin/bash
#
# Post-install script for the Snowflake ODBC driver RPM.
# Runs on the customer's machine after "rpm -i".
#
# Registers the driver with unixODBC and creates a template DSN.
#

if [[ -z "$SF_ACCOUNT" ]]; then
    echo "[WARN] SF_ACCOUNT is not set, please manually update the odbc.ini file after installation"
    SF_ACCOUNT=SF_ACCOUNT
fi

ODBC_DIR=/usr/lib64/snowflake/odbc

echo "Adding driver info to odbcinst.ini..."
odbcinst -i -d -r <<ODBCINST_INI
[SnowflakeDSIIDriver]
APILevel=1
ConnectFunctions=YYY
Description=Snowflake DSII
Driver=$ODBC_DIR/lib/libsfodbc.so
DriverODBCVer=03.52
SQLLevel=1
ODBCINST_INI

echo "Adding connect info to odbc.ini..."
odbcinst -i -s -l -r <<ODBC_INI
[snowflake]
Description=SnowflakeDB
Driver=SnowflakeDSIIDriver
Locale=en-US
SERVER=$SF_ACCOUNT.snowflakecomputing.com
PORT=443
SSL=on
ACCOUNT=$SF_ACCOUNT
ODBC_INI
