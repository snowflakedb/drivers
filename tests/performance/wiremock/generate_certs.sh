#!/bin/bash
# Generate self-signed CA certificate for WireMock HTTPS proxy functionality.
#
# WireMock acts as an HTTPS man-in-the-middle (MITM) proxy to record and replay
# HTTP traffic between drivers and Snowflake:
#
#   Driver → WireMock (HTTPS proxy) → Snowflake
#
# This certificate enables WireMock to:
#   1. Accept HTTPS connections from drivers (act as HTTPS server)
#   2. Intercept and decrypt HTTPS traffic for recording/replay
#   3. Proxy requests to Snowflake with SSL/TLS
#
# Certificate validation is disabled in drivers.


set -e

CERTS_DIR="/certs"

echo "Generating static CA certificate..."

mkdir -p "$CERTS_DIR"

# Generate self-signed certificate
openssl req -x509 -newkey rsa:2048 -utf8 -days 36500 -nodes \
    -keyout "$CERTS_DIR/wiremock.key" \
    -out "$CERTS_DIR/wiremock.crt" \
    -subj "/CN=WireMock Static CA" \
    -addext "basicConstraints=CA:TRUE" \
    -addext "keyUsage=keyCertSign,cRLSign"

# Convert to PKCS12 format
openssl pkcs12 -export \
    -inkey "$CERTS_DIR/wiremock.key" \
    -in "$CERTS_DIR/wiremock.crt" \
    -out "$CERTS_DIR/wiremock.p12" \
    -password "pass:password" \
    -nomac

# Convert to JKS format for WireMock
keytool -importkeystore \
    -deststorepass password \
    -destkeypass password \
    -srckeystore "$CERTS_DIR/wiremock.p12" \
    -srcstorepass password \
    -deststoretype jks \
    -destkeystore "$CERTS_DIR/wiremock.jks"

# Clean up intermediate files
rm "$CERTS_DIR/wiremock.p12" "$CERTS_DIR/wiremock.key"

echo "✓ Static CA certificate generated at $CERTS_DIR/wiremock.jks"
