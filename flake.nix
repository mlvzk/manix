{
  description = "A fast CLI documentation searcher for Nix.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem flake-utils.lib.allSystems (system: {
      packages.manix = nixpkgs.legacyPackages.${system}.callPackage ./. {};
      defaultPackage = self.packages.${system}.manix;
    });
}
