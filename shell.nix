{ pkgs ? import <nixpkgs> {}
}: let
  unstable = import <unstable> {};
in pkgs.mkShell {
  buildInputs = [ unstable.rust-analyzer unstable.cargo-flamegraph ];
}
