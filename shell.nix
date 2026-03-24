{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.cmake
    pkgs.rustup
  ];

  buildInputs = [
    pkgs.openssl
    pkgs.unixODBC
    pkgs.zlib
  ];
}
