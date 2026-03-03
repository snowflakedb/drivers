#!/bin/bash

set -euo pipefail

echo "--- Generating proto (Rust and Python are generated on the fly) ---"
cargo run --bin proto_generator -- --generator java --input protobuf/database_driver_v1.proto --output jdbc/src/main/java/
