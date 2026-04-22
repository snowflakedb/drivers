# Build with: cd tests/performance && hatch run build-python
ARG BUILDPLATFORM=linux/amd64

# ============================================================================
# Base stage: OS packages and test application (shared by both drivers)
# ============================================================================
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/python:3.13-slim AS base

RUN apt-get update && apt-get install -y \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    golang-go \
    curl \
    clang \
    unzip \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

# Copy test application
COPY tests/performance/drivers/python/app/ /workdir/

ENV PYTHONUNBUFFERED=1

# ============================================================================
# Universal driver: Cython extensions + pre-built Rust core
# ============================================================================
FROM base AS universal

# Install Rust toolchain (needed for proto_generator during pip install)
COPY rust-toolchain.toml ./rust-toolchain.toml
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup show

# TODO SNOW-2705435: Use Clang for aws-lc-fips-sys to avoid GCC >= 14 incompatibility
ENV AWS_LC_FIPS_SYS_CC=clang

# Copy Rust workspace sources (all members required for cargo workspace resolution)
COPY Cargo.toml Cargo.lock build_common.rs ./
COPY sf_core/ ./sf_core/
COPY sf_mini_core/ ./sf_mini_core/
COPY sf_mini_core_static/ ./sf_mini_core_static/
COPY odbc/ ./odbc/
COPY proto_utils/ ./proto_utils/
COPY proto_generator/ ./proto_generator/
COPY protobuf/ ./protobuf/
COPY error_trace/ ./error_trace/
COPY error_trace_derive/ ./error_trace_derive/
COPY jdbc_bridge/ ./jdbc_bridge/
COPY odbc_trace_tool/ ./odbc_trace_tool/

# Copy Python package sources
COPY python/ ./python/
# Recreate symlinks (Docker COPY does not preserve symlinks to outside the context)
RUN cd python && \
    rm -f Cargo.toml sf_core proto_utils proto_generator protobuf error_trace error_trace_derive && \
    ln -s ../Cargo.toml Cargo.toml && \
    ln -s ../sf_core sf_core && \
    ln -s ../proto_utils proto_utils && \
    ln -s ../proto_generator proto_generator && \
    ln -s ../protobuf protobuf && \
    ln -s ../error_trace error_trace && \
    ln -s ../error_trace_derive error_trace_derive

# Use pre-built libsf_core.so from sf-core-builder.
COPY tests/performance/.tmp/libsf_core.so /workdir/python/src/snowflake/connector/_core/libsf_core.so
COPY tests/performance/.tmp/rust_version /etc/rust_version

# Install the driver — builds Cython extensions via hatch_build.py.
# SKIP_CORE_BUILD tells hatch to skip cargo build of sf_core (we use the pre-built one above).
ENV SKIP_CORE_BUILD=1
RUN pip install --no-cache-dir /workdir/python/

ENV DRIVER_TYPE=universal

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$CA_BUNDLE" && cat "$WIREMOCK_CA_CERT" >> "$CA_BUNDLE"; \
    fi && \
    python main.py

# ============================================================================
# Old driver
# ============================================================================
FROM base AS old

RUN pip install --no-cache-dir snowflake-connector-python
RUN echo "N/A" > /etc/rust_version

ENV DRIVER_TYPE=old

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$CA_BUNDLE" && cat "$WIREMOCK_CA_CERT" >> "$CA_BUNDLE"; \
        export REQUESTS_CA_BUNDLE="$CA_BUNDLE"; \
        export SSL_CERT_FILE="$CA_BUNDLE"; \
    fi && \
    python main.py
