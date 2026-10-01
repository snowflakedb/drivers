FROM rockylinux:8

# perl-Time-Piece, perl-IPC-Cmd and perl-Data-Dumper are required by
# openssl-src's Configure for the vendored-openssl build; unzip extracts the
# protoc release the sf_core build script downloads.
RUN dnf install -y \
    gcc \
    gcc-c++ \
    make \
    cmake \
    jq \
    unzip \
    perl \
    perl-Time-Piece \
    perl-IPC-Cmd \
    perl-Data-Dumper \
  && dnf clean all

ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain none
