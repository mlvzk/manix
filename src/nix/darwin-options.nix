let
  inherit (import <nixpkgs> {}) runCommandLocal;

  nix-darwin = let
    inherit (builtins.tryEval <darwin>) success value;
  in if success then value else fetchTarball https://github.com/LnL7/nix-darwin/archive/refs/heads/master.tar.gz;

  eval = import nix-darwin { configuration = ({ ... }: { }); };
  opts = eval.config.system.build.manual.optionsJSON;
in
runCommandLocal "options.json" { inherit opts; } ''
  cp $opts/share/doc/darwin/options.json $out
''
