{
  description = "Po Co's Wayland compositor";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      crane,
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem =
        { system, pkgs, ... }:
        {
          packages.default = pkgs.callPackage ./nix/packages.nix { inherit crane; };
          devShells.default = pkgs.callPackage ./nix/shell.nix { inherit crane; };
        };
    };
}
