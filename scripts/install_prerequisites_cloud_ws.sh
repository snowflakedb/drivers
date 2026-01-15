#!/bin/bash

set -e

echo "--- Installing system dependencies ---"
sudo dnf config-manager --set-enabled crb
sudo dnf install -y openssl-devel pkg-config cmake unixODBC-devel
echo "--- System dependencies installed ---"

echo "--- Installing Rust ---"
if ! command -v rustc &> /dev/null; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y
fi
echo "--- Rust installed ---"
