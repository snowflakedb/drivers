# syntax=docker/dockerfile:1
#
# SPCS auth e2e probe image.
#
# Stage 1 builds the `spcs_probe` binary from the workspace. Stage 2 is a slim
# runtime carrying just the binary + CA certificates. The image runs as a
# Snowpark Container Services job service; the probe authenticates using the
# platform-injected session token (no user) and runs a query.
#
# Build context is the repository root (see build.sh):
#   docker build -f tests/docker/spcs/Dockerfile -t <tag> .
#
# Built linux/arm64 to match the SPCS compute pool's ARM instance family
# (GEN_ARM_G1_2 in ci/account_setup.sql).

ARG RUST_VERSION=1.88

FROM rust:${RUST_VERSION}-bookworm AS builder

# aws-lc-rs (rustls backend) needs a C toolchain + cmake; sf_core build.rs needs
# pkg-config/openssl headers.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir
COPY . .

RUN cargo build --release --package sf_core --bin spcs_probe \
    && cp target/release/spcs_probe /usr/local/bin/spcs_probe

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/spcs_probe /usr/local/bin/spcs_probe

ENTRYPOINT ["/usr/local/bin/spcs_probe"]
