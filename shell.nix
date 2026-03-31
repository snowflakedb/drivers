{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.cmake
    pkgs.rustup
    pkgs.protobuf_32
    pkgs.python313
    pkgs.uv
  ];

  buildInputs = [
    pkgs.openssl
    pkgs.unixODBC
    pkgs.zlib
  ];

  shellHook = ''
    if [ ! -d "python/.venv" ] || [ ! -f "python/.venv/bin/activate" ]; then
      uv venv "python/.venv" --python python3.13
    fi
    source "python/.venv/bin/activate"
  '';
}
