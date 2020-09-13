{ pkgs ? import <nixpkgs> {}
}: pkgs.rustPlatform.buildRustPackage rec {
  pname = "manix";
  version = "0.6.0";

  src = ./.;
  cargoSha256 = "078v4wgblc1cr5d1hk42i7qac1865bnm26cxxlvc6aya40ryxbv3";
}
