{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustc
    pkgs.cargo
    pkgs.rustfmt
    pkgs.clippy
    pkgs.rust-analyzer

    pkgs.pkg-config
    pkgs.openssl
    pkgs.zlib

    pkgs.nodejs_24
  ];

  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
}
