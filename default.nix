{ pkgs ? import <nixpkgs> {}
}: {
  manix = pkgs.rustPlatform.buildRustPackage rec {
    pname = "manix";
    version = "0.1.0";
    src = ./.;
    cargoSha256 = "14ym6bigwwl2n5fr5b0ak9309pndrrni2p6hly9s2zj7j09hp8vv";
  };
}
