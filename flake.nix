{
  description = "Soundcloud-tui Flake";

  nixConfig = {
    extra-substituters = [
      "https://soundcloud-tui.cachix.org"
    ];
    extra-trusted-public-keys = [
      "soundcloud-tui.cachix.org-1:pWa0zV2eDiZ3qENF0ysBnNNGKH4xBcuYk2fu5RHl41k="
    ];
  };

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.callPackage ./package.nix { };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.stdenv.cc
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
          ];
        };

             }
    );
}
