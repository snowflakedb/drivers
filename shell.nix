{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (fetchTarball {
        url = "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
      }))
    ];
  }
}:

let
  rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
in
pkgs.mkShell {
  nativeBuildInputs = [
    rustToolchain
    pkgs.pkg-config
    pkgs.cmake
    pkgs.protobuf_32
    pkgs.python313
    pkgs.uv
  ];

  buildInputs = [
    pkgs.openssl
    pkgs.unixodbc
    pkgs.zlib
  ];

  shellHook = ''
    if [ ! -d "python/.venv" ] || [ ! -f "python/.venv/bin/activate" ]; then
      uv venv "python/.venv" --python python3.13
    fi
    source "python/.venv/bin/activate"
  '';
}
