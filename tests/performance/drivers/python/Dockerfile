# Build with: cd tests/performance && hatch run build-python
ARG BUILDPLATFORM=linux/amd64

FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/python:3.13-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

# Copy and install python package
COPY python/ /workdir/python/

# Copy the pre-built sf_core library and rust version
RUN mkdir -p /workdir/python/src/snowflake/ud_connector/_core
COPY tests/performance/.tmp/libsf_core.so /workdir/python/src/snowflake/ud_connector/_core/libsf_core.so
COPY tests/performance/.tmp/rust_version /etc/rust_version

RUN pip install --no-cache-dir /workdir/python/

# Copy test application
COPY tests/performance/drivers/python/app/ /workdir/

# Install test dependencies (includes old driver)
RUN pip install --no-cache-dir -r requirements.txt

# Detect OS version at build time (major version only) and write to file
RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

# Set environment variables
ENV PYTHONUNBUFFERED=1

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && python main.py
