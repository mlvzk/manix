# Manix

A fast CLI documentation searcher for Nix.

## Supported sources:

- Nixpkgs Documentation
- Nixpkgs Comments
- Nixpkgs Tree (pkgs., pkgs.lib.)
- NixOS Options
- Home-Manager Options

## Usage

```sh
manix --help
manix mergeattr
manix --strict mergeattr
manix --update-cache mergeattr
```

### rnix-lsp

If you want to use it in your editor, check [ElKowar's rnix-lsp fork](https://github.com/elkowar/rnix-lsp), which uses it to provide documentation on hover and autocompletion.

![manix](/manix.png)

### fzf

```sh
manix "" | grep '^# ' | sed 's/^# \(.*\) (.*/\1/;s/ (.*//;s/^# //' | fzf --preview="./target/release/manix '{}'" | xargs manix
```

## Installation

### Nix

```sh
nix-env -i -f https://github.com/mlvzk/manix/archive/master.tar.gz
```

## Kudos

The inspiration for this project came from [nix-doc](https://github.com/lf-/nix-doc)
