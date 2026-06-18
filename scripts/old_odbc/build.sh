#!/bin/bash

# Build old ODBC driver with coverage enabled.
# This script is meant to run inside the rocky-old-odbc-build container.

set -e

cd /workspace/old_odbc
ODBC_CODE_COVERAGE=1 ./Installer/gen_unix_installer.sh -r -t Release -p
