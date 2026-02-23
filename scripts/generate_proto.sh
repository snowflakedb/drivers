#!/bin/bash

set -e

echo "--- Installing mypy-protobuf for Python .pyi stub generation ---"
pip install "mypy-protobuf==5.0.0" 2>/dev/null || pip3 install "mypy-protobuf==5.0.0" 2>/dev/null || pipx install "mypy-protobuf==5.0.0" 2>/dev/null || echo "Warning: could not install mypy-protobuf, will fall back to protoc --pyi_out"

echo "--- Generating proto (Rust is generated on the fly via build.rs) ---"
cargo run --bin proto_generator -- --generator python --input protobuf/database_driver_v1.proto --output python/src/snowflake/connector/_internal/protobuf_gen/
cargo run --bin proto_generator -- --generator java --input protobuf/database_driver_v1.proto --output jdbc/src/main/java/

