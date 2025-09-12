FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y unixodbc unixodbc-dev curl odbcinst wget build-essential git gdb \
    zlib1g-dev

# Install CMake
RUN wget https://github.com/Kitware/CMake/releases/download/v4.0.3/cmake-4.0.3-linux-aarch64.sh && \
    sh cmake-4.0.3-linux-aarch64.sh --skip-license --prefix=/usr/local && \
    rm cmake-4.0.3-linux-aarch64.sh && \
    cmake --version

# Install Snowflake ODBC driver
RUN curl -O https://sfc-repo.snowflakecomputing.com/odbc/linuxaarch64/3.11.0/snowflake-odbc-3.11.0.aarch64.deb && \
    dpkg -i snowflake-odbc-3.11.0.aarch64.deb && \
    sed -i 's,ODBCInstLib=.*,ODBCInstLib=/usr/lib/aarch64-linux-gnu/libodbcinst.so.2,g' /usr/lib/snowflake/odbc/lib/simba.snowflake.ini