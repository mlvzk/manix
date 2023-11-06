with import <nixpkgs> {}; let
  eval = import (pkgs.path + "/nixos/lib/eval-config.nix") {modules = [];};
  opts = (nixosOptionsDoc {options = eval.options;}).optionsJSON;
in
  runCommandLocal "options.json" {inherit opts;}
  "cp $opts/share/doc/nixos/options.json $out"
