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
COPY nodejs_bridge/ ./nodejs_bridge/
COPY odbc_trace_tool/ ./odbc_trace_tool/
COPY python_bridge/ ./python_bridge/

# Copy Python package sources
COPY python/ ./python/
# Recreate symlinks (Docker COPY does not preserve symlinks to outside the context)
RUN cd python && \
    rm -f Cargo.toml sf_core proto_utils proto_generator protobuf error_trace error_trace_derive python_bridge && \
    ln -s ../Cargo.toml Cargo.toml && \
    ln -s ../sf_core sf_core && \
    ln -s ../proto_utils proto_utils && \
    ln -s ../proto_generator proto_generator && \
    ln -s ../protobuf protobuf && \
    ln -s ../error_trace error_trace && \
    ln -s ../error_trace_derive error_trace_derive && \
    ln -s ../python_bridge python_bridge

# Install the driver — builds Cython extensions + python_bridge via hatch_build.py.
# SF_PERF_METRICS enables sf_core/perf_timing in the PyO3 extension build.
ENV SF_PERF_METRICS=1
RUN pip install --no-cache-dir "/workdir/python/[pandas]"

# Record the Rust toolchain that built python_bridge (for results metadata).
RUN rustc --version | awk '{print $2}' | cut -d. -f1,2 > /etc/rust_version

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

RUN pip install --no-cache-dir "snowflake-connector-python[pandas]"
RUN echo "N/A" > /etc/rust_version

ENV DRIVER_TYPE=old

# TLS trust for the WireMock test proxy is not needed here: the app relaxes
# certificate verification for the old driver behind the proxy at runtime instead.
CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    python main.py
