{ sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs {}
}: let
  rust_manix = pkgs.rustPlatform.buildRustPackage rec {
    pname = "rust_manix";
    version = "0.6.0";
    src = ./.;
    cargoSha256 = "0ajl1xc7n3m4gvqrs254381imis8yri4dyksixjq1psxkwvwaf7f";
  };
in pkgs.linkFarmFromDrvs "manix" [ rust_manix ]
/*pkgs.stdenv.mkDerivation {
  name = "manix";
  version = "0.6.0";
  buildInputs = [ rust_manix ];
  noBuild = true;
}*/
