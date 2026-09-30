FROM node:22-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    clang \
    curl \
    pkg-config \
    libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# npm 10.x (the version bundled with Node 22) crashes on `npm install`
# with "Cannot read properties of null (reading 'edgesOut')". npm 11
# does not hit it. https://github.com/npm/cli/issues/9787
# TODO: Remove this once npm fixes the bug, so the build runs on the npm that
# ships with Node — matching how most users install the driver.
RUN npm install -g npm@11

ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain none
