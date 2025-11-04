# Build with: cd tests/performance && hatch run build-core

ARG BUILDPLATFORM=linux/amd64

# Stage 1: Planner - generate recipe for dependencies
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS planner

# Install cargo-chef
RUN cargo install cargo-chef

WORKDIR /workdir

# Copy all source files needed for recipe generation
COPY rust-toolchain.toml ./rust-toolchain.toml
COPY build_common.rs ./build_common.rs
COPY sf_core/ ./sf_core/
COPY proto_utils/ ./proto_utils/
COPY tests/performance/drivers/core/app/ ./tests/performance/drivers/core/app/

# Modify sf_core/Cargo.toml to only build rlib (not dylib) to avoid linker issues
RUN sed -i 's/crate-type = \["dylib", "rlib"\]/crate-type = ["rlib"]/' ./sf_core/Cargo.toml

WORKDIR /workdir/tests/performance/drivers/core/app
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder - build dependencies (cached) then app
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    golang-go \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef
RUN cargo install cargo-chef

# todo SNOW-2705435 fix aws-lc-fips-sys build on ARM64
ENV CC=gcc

WORKDIR /workdir

# Copy rust-toolchain.toml and install correct Rust version
COPY rust-toolchain.toml ./rust-toolchain.toml
RUN rustup show

# Copy build_common.rs (needed by sf_core build)
COPY build_common.rs ./build_common.rs

# Copy sf_core and proto_utils source (needed for path dependencies)
COPY sf_core/ ./sf_core/
COPY proto_utils/ ./proto_utils/

# Modify sf_core/Cargo.toml to only build rlib (not dylib) to avoid linker issues
RUN sed -i 's/crate-type = \["dylib", "rlib"\]/crate-type = ["rlib"]/' ./sf_core/Cargo.toml

WORKDIR /workdir/tests/performance/drivers/core/app

# Build dependencies - this layer will be cached unless dependencies change
COPY --from=planner /workdir/tests/performance/drivers/core/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy application source and build only the app (sf_core already compiled)
COPY tests/performance/drivers/core/app/ ./

RUN cargo build --release --bin core-perf-driver

# Stage 3: Runtime - use minimal base image with newer GLIBC
ARG BUILDPLATFORM=linux/amd64
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /workdir/tests/performance/drivers/core/app/target/release/core-perf-driver /usr/local/bin/core-perf-driver

# Detect OS version at build time (major version only) and write to file
RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

ENV RUST_LOG=${RUST_LOG:-warn}

CMD export OS_INFO=$(cat /etc/os_info) && core-perf-driver
