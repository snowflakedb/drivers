# Build with: cd tests/performance && hatch run build-python
ARG BUILDPLATFORM=linux/amd64

FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/python:3.13-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

# Copy the pre-built sf_core library
COPY tests/performance/.tmp_libsf_core.so /usr/lib/libsf_core.so

# Copy and install pep249_dbapi package
COPY pep249_dbapi/ /workdir/pep249_dbapi/
RUN pip install --no-cache-dir /workdir/pep249_dbapi/

# Copy test application
COPY tests/performance/drivers/python/app/ /workdir/

# Install test dependencies (includes old driver)
RUN pip install --no-cache-dir -r requirements.txt

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV CORE_PATH=/usr/lib/libsf_core.so

# Run the application
CMD ["python", "main.py"]
