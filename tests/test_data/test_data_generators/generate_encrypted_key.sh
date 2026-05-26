#!/usr/bin/env bash
# Generates the encrypted test key used by ODBC integration tests.
# Re-run this if you change invalid_rsa_key.p8 or the test password.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$(dirname "$SCRIPT_DIR")"

openssl pkey \
  -in "$DATA_DIR/invalid_rsa_key.p8" \
  -out "$DATA_DIR/invalid_rsa_key_encrypted.p8" \
  -aes-256-cbc \
  -passout pass:test_password_123
