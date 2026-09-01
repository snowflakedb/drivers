# Build with: cd tests/performance && hatch run build-nodejs
#
# The universal stage builds the nodejs/ package via its own documented
# 3-step pipeline (build:core -> build:sdk -> build:link-local). That pipeline
# expects nodejs/ to sit inside a real checkout, next to its sibling Rust
# crates (nodejs_bridge/, sf_core/, ...) -- so this copies the whole repo in
# rather than reconstructing that layout crate-by-crate.
ARG BUILDPLATFORM=linux/amd64

# ============================================================================
# Base stage: OS packages and test application (shared by both drivers)
# ============================================================================
FROM --platform=${BUILDPLATFORM} artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/development-docker-virtual/node:22-slim AS base

RUN apt-get update && apt-get install -y \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workdir

RUN bash -c '. /etc/os-release && echo "${ID^}_$(echo $VERSION_ID | cut -d. -f1)" > /etc/os_info'

# NOTE: app/ is intentionally copied at the end of each stage below, not here.
# In the "old" stage this keeps app-only edits from invalidating the
# `npm install snowflake-sdk` layer. The "universal" stage's own `COPY . .`
# already includes app/ as part of the whole repo, so app-only edits
# invalidate its Rust/napi build regardless -- the late copy there exists to
# place app/ in its own subdirectory under the built package (see that
# stage's own comment for why it can't share nodejs/'s own directory).

# ============================================================================
# Universal driver: build the napi native addon from source, install the
# local nodejs/ consumer package under the name `snowflake-sdk`
# ============================================================================
FROM base AS universal

# sf_core's build.rs downloads protoc from GitHub releases by default
# (proto_generator/src/protoc_installer.rs). Install it via apt instead and
# point PROTOC at it -- protoc_installer.rs explicitly honors this env var as
# an "offline builds" override, which sidesteps flaky/restricted outbound
# GitHub access during this build.
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
ENV PROTOC=/usr/bin/protoc

# Install Rust toolchain (nodejs_bridge is a Rust/napi crate)
COPY rust-toolchain.toml ./rust-toolchain.toml
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup show

# Copy the whole repo -- nodejs/'s build scripts resolve sibling Rust crates
# (nodejs_bridge/, sf_core/, ...) relative to their own location, so they need
# the real checkout layout, not a hand-picked subset. .dockerignore already
# excludes node_modules/, _build/, target/, and .git/.
COPY . .
WORKDIR /workdir/nodejs

# 1. npm install (--include=dev: build:core/build:sdk/build:link-local need
#    @napi-rs/cli/typescript, all devDependencies)
# 2. build:core compiles nodejs_bridge into a platform-specific native package
#    under _build/snowflake-sdk-core/ (see scripts/build-snowflake-sdk-core.mjs)
# 3. build:sdk compiles src/ into a publish-ready package under
#    _build/snowflake-sdk/ (see scripts/build-snowflake-sdk.mjs) -- the same
#    package nodejs's own e2e tests and npm publish use
# 4. build:link-local npm-links both built packages into this package's own
#    node_modules (see scripts/build-link-local.mjs), so require('snowflake-sdk')
#    resolves here without any manual node_modules assembly
RUN npm install --include=dev
RUN npm run build:core
RUN npm run build:sdk
RUN npm run build:link-local

RUN echo "$(rustc --version | awk '{print $2}' | cut -d. -f1,2)" > /etc/rust_version

# Copy test application last -- see NOTE on the base stage. app/ gets its own
# subdirectory (not directly in /workdir/nodejs) because nodejs/'s
# package.json declares "type": "module", which would otherwise make Node
# treat these CommonJS (require()/module.exports) files as ES modules --
# app/package.json overrides that back to commonjs for this subtree, while
# node_modules resolution still walks up to /workdir/nodejs/node_modules.
COPY tests/performance/drivers/nodejs/app/ /workdir/nodejs/app/
WORKDIR /workdir/nodejs/app

ENV DRIVER_TYPE=universal

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    node main.js

# ============================================================================
# Old driver: published npm package
# ============================================================================
FROM base AS old

RUN npm install snowflake-sdk@^3.1.0
RUN echo "N/A" > /etc/rust_version

# Copy test application last -- see NOTE on the base stage.
COPY tests/performance/drivers/nodejs/app/ /workdir/

ENV DRIVER_TYPE=old

# Node's default V8 old-space-size heuristic caps well below the container's
# actual mem_limit (runner/container.py's MEMORY_LIMIT_OVERRIDES sets 8192m
# for this (nodejs, old) pair specifically -- every other driver/case keeps
# the shared 4096m default). The old driver's non-streaming execute() path
# buffers the entire result set into a JS array before returning
# (statement.js's rows.push(row)), and being pure JS, wire-format parsing +
# intermediate buffers + that final array all share the same heap. For wide
# cases like 15columns (15 columns incl. a free-text comment field) x 1M rows,
# real heap demand exceeds even a 3584MB ceiling under a 4096m container.
# Raise both the container's memory (above, in container.py) and this heap
# ceiling together, leaving ~1GB headroom under the container limit for
# non-heap/native memory.
ENV NODE_OPTIONS="--max-old-space-size=7168"

CMD export OS_INFO=$(cat /etc/os_info) && export BUILD_RUST_VERSION=$(cat /etc/rust_version) && \
    node main.js
