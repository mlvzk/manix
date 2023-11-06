{
  release ? "23.05",
  isReleaseBranch ? false,
  pkgs ? import <nixpkgs> {},
}: let
  hmargs = {
    inherit release isReleaseBranch pkgs;
    lib = import <home-manager/modules/lib/stdlib-extended.nix> pkgs.lib;
  };

  docs = import <home-manager/docs> hmargs;
in
  (
    if builtins.isFunction docs
    then docs hmargs
    else docs
  )
  .options
  .json
