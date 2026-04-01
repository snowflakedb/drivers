FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y unixodbc unixodbc-dev curl odbcinst wget build-essential git gdb \
    zlib1g-dev libssl-dev jq ccache

# Install CMake (detect architecture automatically)
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then CMAKE_ARCH="x86_64"; \
    else CMAKE_ARCH="aarch64"; fi && \
    wget "https://github.com/Kitware/CMake/releases/download/v4.0.3/cmake-4.0.3-linux-${CMAKE_ARCH}.sh" && \
    sh "cmake-4.0.3-linux-${CMAKE_ARCH}.sh" --skip-license --prefix=/usr/local && \
    rm "cmake-4.0.3-linux-${CMAKE_ARCH}.sh" && \
    cmake --version

# Install Snowflake ODBC driver (detect architecture automatically)
# Note: Snowflake repo uses "linux" for x86_64 and "linuxaarch64" for aarch64
ARG REFERENCE_ODBC_VERSION
ARG REFERENCE_ODBC_SHA256
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        PKG_ARCH="x86_64"; URL_PATH="linux"; \
    else \
        PKG_ARCH="aarch64"; URL_PATH="linuxaarch64"; \
    fi && \
    DEB="snowflake-odbc-${REFERENCE_ODBC_VERSION}.${PKG_ARCH}.deb" && \
    curl -O "https://sfc-repo.snowflakecomputing.com/odbc/${URL_PATH}/${REFERENCE_ODBC_VERSION}/${DEB}" && \
    echo "${REFERENCE_ODBC_SHA256}  ${DEB}" | sha256sum -c - && \
    dpkg -i "${DEB}" && \
    rm -f "${DEB}" && \
    ODBC_LIB_DIR=$(find /usr/lib -name 'libodbcinst.so.2' -printf '%h' -quit) && \
    sed -i "s,ODBCInstLib=.*,ODBCInstLib=${ODBC_LIB_DIR}/libodbcinst.so.2,g" /usr/lib/snowflake/odbc/lib/simba.snowflake.ini