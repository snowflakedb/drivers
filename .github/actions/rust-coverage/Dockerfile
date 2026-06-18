FROM ubuntu:22.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies for Rust and building the cargo-tarpaulin
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    unixodbc-dev \
    build-essential \
    pkg-config \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Rust components
RUN rustup component add rustfmt
RUN cargo install cargo-tarpaulin --version 0.32.8

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"] 