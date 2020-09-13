{ sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs {}
}: (pkgs.callPackage ./Cargo.nix {}).rootCrate.build
