{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell {
  CARGO_MANIFEST_DIR=toString ./.;
  buildInputs = [
    cargo rustc rust-analyzer rustfmt clippy
    dbus openssl pkg-config
  ];
}
