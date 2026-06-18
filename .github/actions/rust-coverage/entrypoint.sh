#!/bin/sh -l

set -e

echo "Running code coverage"
rustup default 1.87.0
cargo tarpaulin --workspace --out Lcov --output-dir ./coverage/ 
