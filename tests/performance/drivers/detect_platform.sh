#!/bin/bash
# Shared script to detect build platform architecture
# Usage: source this script to set BUILDPLATFORM variable
#
# If BUILDPLATFORM is already set (e.g., from CI), it will be preserved.
# Otherwise, it auto-detects based on the host architecture.

if [ -z "${BUILDPLATFORM}" ]; then
    ARCH=$(uname -m)
    case "${ARCH}" in
        x86_64)
            BUILDPLATFORM="linux/amd64"
            ;;
        aarch64|arm64)
            BUILDPLATFORM="linux/arm64"
            ;;
        *)
            echo "⚠ Warning: Unknown architecture ${ARCH}, defaulting to linux/amd64"
            BUILDPLATFORM="linux/amd64"
            ;;
    esac
    echo "Auto-detected platform: ${BUILDPLATFORM}"
fi

export BUILDPLATFORM
