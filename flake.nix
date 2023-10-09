{
  description = "A fast CLI documentation searcher for Nix.";

  inputs.naersk.url = "github:nix-community/naersk";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = {
    self,
    naersk,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachSystem flake-utils.lib.allSystems (system: let
      pkgs = (import nixpkgs) {
        inherit system;
      };

      naersk' = pkgs.callPackage naersk {};
    in {
      defaultPackage = self.packages.${system}.manix;
      packages.manix = naersk'.buildPackage {
        src = ./.;
      };

      # For `nix develop` (optional, can be skipped):
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [rustc cargo];
      };
    });
}
