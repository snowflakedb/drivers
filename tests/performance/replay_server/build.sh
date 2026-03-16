#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building replay server container..."

docker build -t replay-server:latest .

echo "✓ Replay server container built: replay-server:latest"
