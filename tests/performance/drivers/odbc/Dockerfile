# Build with: cd tests/performance && hatch run build-odbc

ARG BUILDPLATFORM=linux/amd64

# ============================================================================
# Stage: Download and install old Snowflake ODBC driver
# ============================================================================
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS old_installer

RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

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

# ============================================================================
# Base stage: runtime deps + test app build (shared)
# ============================================================================
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/rust:slim AS base

RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    unixodbc \
    unixodbc-dev \
    ca-certificates \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

COPY tests/performance/drivers/odbc/app/ /workdir/

RUN mkdir -p build && \
    cd build && \
    cmake .. && \
    cmake --build .

RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

ENV LD_LIBRARY_PATH=/usr/lib
ENV ODBCINI=/workdir/odbc.ini
ENV ODBCSYSINI=/workdir

# ============================================================================
# Universal driver image
# ============================================================================
FROM base AS universal

COPY tests/performance/.tmp/libsfodbc.so /usr/lib/libsfodbc.so
COPY tests/performance/.tmp/libsf_core.so /usr/lib/libsf_core.so
COPY tests/performance/.tmp/rust_version /etc/rust_version

ENV DRIVER_TYPE=universal

CMD export OS_INFO=$(cat /etc/os_info) && export RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        SYS_CA=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$SYS_CA" && cat "$WIREMOCK_CA_CERT" >> "$SYS_CA"; \
    fi && \
    /workdir/build/odbc-perf-driver

# ============================================================================
# Old driver image
# ============================================================================
FROM base AS old

COPY --from=old_installer /usr/lib/snowflake/ /usr/lib/snowflake/
RUN echo "N/A" > /etc/rust_version

ENV DRIVER_TYPE=old

CMD export OS_INFO=$(cat /etc/os_info) && export RUST_VERSION=$(cat /etc/rust_version) && \
    if [ -n "$WIREMOCK_CA_CERT" ] && [ -f "$WIREMOCK_CA_CERT" ]; then \
        SYS_CA=/etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> "$SYS_CA" && cat "$WIREMOCK_CA_CERT" >> "$SYS_CA"; \
        CACERT=/usr/lib/snowflake/odbc/lib/cacert.pem; \
        printf '\n' >> "$CACERT" && cat "$WIREMOCK_CA_CERT" >> "$CACERT"; \
        INI_ORIG=/usr/lib/snowflake/odbc/lib/simba.snowflake.ini; \
        INI=/tmp/simba.snowflake.wiremock.ini; \
        if [ -f "$INI_ORIG" ]; then cp "$INI_ORIG" "$INI"; else : > "$INI"; fi; \
        printf '\n[Driver]\nCABundleFile=%s\nDisableOCSPCheck=true\nErrorMessagesPath=/usr/lib/snowflake/odbc/ErrorMessages/\nODBCInstLib=libodbcinst.so\n' "$CACERT" >> "$INI"; \
        export SIMBAINI="$INI"; \
    fi && \
    /workdir/build/odbc-perf-driver
