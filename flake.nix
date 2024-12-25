{
  description = "A small program for updating eww variables in sway";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = { url = "github:hercules-ci/gitignore.nix"; flake = false; };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };



  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system: let
        gitignoreSrc = pkgs.callPackage inputs.gitignore {};
        overlays = [(import inputs.rust-overlay)];
        pkgs = import (inputs.nixpkgs) { inherit system overlays; };

        inherit (inputs.nixpkgs) lib;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.stable.latest.minimal;
          rustc = pkgs.rust-bin.stable.latest.minimal;
        };
      in rec {
        #packages.sway_update = pkgs.callPackage ./default.nix { inherit gitignoreSrc; };
        packages.sway_update = rustPlatform.buildRustPackage {
          pname = "sway_update";
          version = "0.1.0";

          src = gitignoreSrc.gitignoreSource ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          meta = with lib; {
            homepage = "";
            description = "A small program for updating eww variables in sway";
            license = licenses.mit;
          };
        };
        packages.default = packages.sway_update;
        devShell = pkgs.mkShell {
          CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";
          buildInputs = with pkgs; [ cargo rustc git ];
        };
      }
    );
}
