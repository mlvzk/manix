{ pkgs ? import <nixpkgs> {}
}: {
  manix = pkgs.rustPlatform.buildRustPackage rec {
    pname = "manix";
    version = "0.1.0";
    src = ./.;
    cargoSha256 = "01vlz3xzkirl0qhllpfha6jbm8ld617fm2j7ypkrva0ysmxbsly0";
  };
}
