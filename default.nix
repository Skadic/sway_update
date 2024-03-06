{ 
  pkgs ? import <nixpkgs> {},
  stdenv ? pkgs.stdenv,
  lib ? stdenv.lib,
  rustPlatform ? pkgs.rustPlatform,
  gitignoreSrc
}:
let 
  gitignoreSource = gitignoreSrc.gitignoreSource;
in
rustPlatform.buildRustPackage {
  pname = "sway_update";
  version = "0.1.0";

  src = gitignoreSource ./.;
  cargoHash = lib.fakeHash;

  meta = with lib; {
    homepage = "";
    description = "A small program for updating eww variables in sway";
    license = licenses.mit;
  };
}
