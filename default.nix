{ pkgs ? import <nixpkgs> {} }:
let
    sources = import ./nix/sources.nix;
    naersk = pkgs.callPackage sources.naersk {};
in naersk.buildPackage ./.
