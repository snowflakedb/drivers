FROM node:20-bookworm

WORKDIR /mnt/host

# Common tools & C/C++ build essentials
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates wget jq \
    build-essential pkg-config libssl-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Browser
RUN apt-get update && apt-get install -y --no-install-recommends \
    chromium \
    && rm -rf /var/lib/apt/lists/*

# Python
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 python3-dev python3-pip python3-pytest python3-venv \
    && rm -rf /var/lib/apt/lists/*

# ODBC
RUN apt-get update && apt-get install -y --no-install-recommends \
    unixodbc unixodbc-dev odbcinst \
    && rm -rf /var/lib/apt/lists/*

# Java
RUN apt-get update && apt-get install -y --no-install-recommends \
    openjdk-17-jdk-headless \
    && rm -rf /var/lib/apt/lists/*

# CMake >= 4.0 (required by odbc_tests CMakeLists)
ARG CMAKE_VERSION=4.0.3
RUN ARCH=$(dpkg --print-architecture) \
    && if [ "$ARCH" = "amd64" ]; then CMAKE_ARCH="x86_64"; else CMAKE_ARCH="aarch64"; fi \
    && wget -q "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}.sh" \
    && sh "cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}.sh" --skip-license --prefix=/usr/local \
    && rm -f "cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}.sh"

# Browser credential helper scripts (Playwright-based auth automation)
COPY browser-helpers/package.json /externalbrowser/package.json
COPY browser-helpers/cleanBrowserProcesses.js /externalbrowser/cleanBrowserProcesses.js
COPY browser-helpers/provideBrowserCredentials.js /externalbrowser/provideBrowserCredentials.js
COPY browser-helpers/getOauthToken.js /externalbrowser/getOauthToken.js
COPY browser-helpers/patHelper.js /externalbrowser/patHelper.js
COPY browser-helpers/totpGenerator.js /externalbrowser/totpGenerator.js
COPY browser-helpers/getTOTP.js /externalbrowser/getTOTP.js

# Headless Chromium launcher
RUN printf '#!/bin/sh\nchromium --no-sandbox --headless --disable-gpu --enable-unsafe-swiftshader --remote-debugging-port=9222 "$@" 2>/dev/null &\n' > /chromium-headless.sh \
    && chmod +x /chromium-headless.sh
ENV BROWSER=/chromium-headless.sh

RUN cd /externalbrowser && npm install

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.88.0
ENV PATH="/root/.cargo/bin:${PATH}"
