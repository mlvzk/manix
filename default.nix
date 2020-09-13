{ sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs {}
}: pkgs.rustPlatform.buildRustPackage rec {
  pname = "manix";
  version = "0.6.0";

  src = ./.;
  cargoSha256 = "0wpc65cl98lh2zdgrwxg07hhvfkmhwkb0xzyg6rd1v6xnh13g01j";
}
