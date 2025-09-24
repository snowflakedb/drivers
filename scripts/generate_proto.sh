#!/bin/bash

set -e

echo "--- Downloading protoc ---"
if [ ! -f bin/protoc/bin/protoc ]; then
    mkdir -p bin/protoc
    
    # Detect OS
    echo "--- Detect OS ---"
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    case $OS in
        "darwin") OS="osx" ;;
        "linux") OS="linux" ;;
        *) echo "Unsupported OS: $OS" && exit 1 ;;
    esac
    echo "OS: $OS"
    
    # Detect architecture
    echo "--- Detect architecture ---"
    ARCH=$(uname -m)
    case $ARCH in
        "x86_64") ARCH="x86_64" ;;
        "aarch64"|"arm64") ARCH="aarch_64" ;;
        *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
    esac
    echo "ARCH: $ARCH"

    PROTOC_ZIP="protoc-32.1-$OS-$ARCH.zip"
    curl -L -o $PROTOC_ZIP https://github.com/protocolbuffers/protobuf/releases/download/v32.1/$PROTOC_ZIP
    unzip $PROTOC_ZIP -d bin/protoc
    rm $PROTOC_ZIP
fi

export PATH=$(pwd)/bin/protoc/bin:$PATH

which protoc

echo "--- Generating proto ---"
cargo run --bin proto_generator -- --generator rust --input protobuf/database_driver_v1.proto --output sf_core/src/protobuf_gen/
cargo run --bin proto_generator -- --generator python --input protobuf/database_driver_v1.proto --output pep249_dbapi/pep249_dbapi/protobuf_gen/
cargo run --bin proto_generator -- --generator java --input protobuf/database_driver_v1.proto --output jdbc/src/java/

