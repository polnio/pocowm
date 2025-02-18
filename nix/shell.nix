{
  # Inputs
  crane,

  # Tools
  callPackage,
  clippy,
  pkg-config,
  pkgs,
  rustPlatform,
}:
let
  craneLib = crane.mkLib pkgs;
  package = callPackage ./packages.nix { inherit crane; };
in
craneLib.devShell {
  packages = [
    clippy
    pkg-config
  ];
  inputsFrom = [ package ];
  RUST_SRC_PATH = rustPlatform.rustLibSrc;
  LD_LIBRARY_PATH = package.LD_LIBRARY_PATH;
}
