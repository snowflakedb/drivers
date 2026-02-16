#!/bin/bash
# Rebuild Rust core and update Python wrapper
set -e

echo "🔨 Building Rust core library..."
cargo build --package sf_core

echo "📦 Copying library to Python package..."
mkdir -p python/src/snowflake/connector/_core
cp target/debug/libsf_core.so python/src/snowflake/connector/_core/

echo "✅ Done! You can now run Python tests:"
echo "   cd python && hatch run dev:unit"
