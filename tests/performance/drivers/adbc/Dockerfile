# Build with: cd tests/performance && hatch run build-adbc
ARG BUILDPLATFORM=linux/amd64

FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/python:3.13-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

COPY tests/performance/drivers/_shared/ /opt/shared/
COPY tests/performance/drivers/adbc/app/ /workdir/

RUN pip install --no-cache-dir \
    "adbc-driver-snowflake>=1.2.0" \
    "pyarrow>=17.0.0"

ENV PYTHONPATH=/opt/shared
ENV PYTHONUNBUFFERED=1

CMD export OS_INFO=$(cat /etc/os_info) && python main.py
