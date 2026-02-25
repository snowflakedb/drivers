# Build with: cd tests/performance && hatch run build-python
ARG BUILDPLATFORM=linux/amd64

FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/python:3.13-slim

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

# Install Rust toolchain
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

# Save Rust version for runtime reporting
RUN rustc --version | awk '{print $2}' | cut -d. -f1,2 > /etc/rust_version

# Install the driver — this builds Rust core and Cython extensions via hatch_build.py
RUN pip install --no-cache-dir /workdir/python/

# Copy test application
COPY tests/performance/drivers/python/app/ /workdir/

# Install old driver to separate directory for comparison tests
# Using -t flag to install to old_driver_src without overwriting universal driver
RUN pip install --no-cache-dir snowflake-connector-python -t /workdir/old_driver_src

# Detect OS version at build time (major version only) and write to file
RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

# Set environment variables
ENV PYTHONUNBUFFERED=1

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && python main.py
