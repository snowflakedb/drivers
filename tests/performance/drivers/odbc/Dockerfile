# Build with: cd tests/performance && hatch run build-odbc

ARG BUILDPLATFORM=linux/amd64

# Stage 1: Install old Snowflake ODBC driver
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS old_installer

RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Download and install Snowflake old ODBC driver (latest version)
RUN if [ "$(uname -m)" = "x86_64" ]; then \
        ARCH="linux"; \
        PACKAGE_ARCH="x86_64"; \
    elif [ "$(uname -m)" = "aarch64" ]; then \
        ARCH="linuxaarch64"; \
        PACKAGE_ARCH="aarch64"; \
    else \
        echo "Unsupported architecture: $(uname -m)"; \
        exit 1; \
    fi && \
    echo "Downloading latest Snowflake ODBC driver for ${ARCH}..." && \
    LATEST_VERSION=$(curl -s https://sfc-repo.snowflakecomputing.com/odbc/${ARCH}/latest/index.html | grep -oP 'snowflake-odbc-\K[0-9]+\.[0-9]+\.[0-9]+' | head -1) && \
    echo "Found version: ${LATEST_VERSION}" && \
    curl -O https://sfc-repo.snowflakecomputing.com/odbc/${ARCH}/${LATEST_VERSION}/snowflake-odbc-${LATEST_VERSION}.${PACKAGE_ARCH}.deb && \
    dpkg -i snowflake-odbc-${LATEST_VERSION}.${PACKAGE_ARCH}.deb || true && \
    rm -f snowflake-odbc-${LATEST_VERSION}.${PACKAGE_ARCH}.deb

# Stage 2: Runtime with both drivers  
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS runtime_base
# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    unixodbc \
    unixodbc-dev \
    ca-certificates \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

FROM runtime_base

WORKDIR /workdir

# Copy the universal ODBC wrapper and sf_core library
COPY tests/performance/.tmp/libsfodbc.so /usr/lib/libsfodbc.so
COPY tests/performance/.tmp/libsf_core.so /usr/lib/libsf_core.so

# Copy Rust version from sf-core-builder
COPY tests/performance/.tmp/rust_version /etc/rust_version

# Copy old driver from installer stage
COPY --from=old_installer /usr/lib/snowflake/ /usr/lib/snowflake/

# Copy ODBC test application
COPY tests/performance/drivers/odbc/app/ /workdir/

# Build ODBC app
RUN mkdir -p build && \
    cd build && \
    cmake .. && \
    cmake --build .

# Detect OS version at build time (major version only) and write to file
RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

# Set environment variables
ENV LD_LIBRARY_PATH=/usr/lib
ENV ODBCINI=/workdir/odbc.ini
ENV ODBCSYSINI=/workdir

# Default to universal driver (can be overridden with DRIVER_TYPE=old)
ENV DRIVER_TYPE=universal

CMD export OS_INFO=$(cat /etc/os_info) && export RUST_VERSION=$(cat /etc/rust_version) && /workdir/build/odbc-perf-driver

