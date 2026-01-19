#!/bin/bash -e

#
# Login to the Internal Docker Registry
#

INTERNAL_REPO="${INTERNAL_REPO:-nexus.int.snowflakecomputing.com:8086}"

echo "[INFO] Login to the internal Docker Registry"
NEXUS_USER=${USERNAME:-${USER:-jenkins}}

if [[ -z "$NEXUS_PASSWORD" ]]; then
    echo "[ERROR] Set NEXUS_PASSWORD to your LDAP password to access the internal repository!"
    exit 1
fi

if ! docker login --username "$NEXUS_USER" --password-stdin "$INTERNAL_REPO" <<< "$NEXUS_PASSWORD"; then
    echo "[ERROR] Failed to connect to the nexus server. Verify the environment variable NEXUS_PASSWORD is set correctly for NEXUS_USER: $NEXUS_USER"
    exit 1
fi

echo "[INFO] Successfully logged in to $INTERNAL_REPO"

