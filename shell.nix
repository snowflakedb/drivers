{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.cmake
    pkgs.rustup
    pkgs.protobuf_33
    pkgs.python312
    pkgs.uv
  ];

  buildInputs = [
    pkgs.openssl
    pkgs.unixODBC
    pkgs.zlib
  ];

  shellHook = ''
    if [ ! -d python/.venv ]; then
      uv venv python/.venv --python python3.12
    fi
    source python/.venv/bin/activate
  '';
}
