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
manix "" | grep '^# ' | sed 's/^# \(.*\) (.*/\1/;s/ (.*//;s/^# //' | fzf --preview="manix '{}'" | xargs manix
```

## Installation

### nix-env

```sh
sudo nix-env \
	--option extra-substituters https://mlvzk.cachix.org/ \
	--option trusted-public-keys 'mlvzk.cachix.org-1:OxRv8xos92A++mh82CsgVo8J6cHGSVCOBFx7a6nhbWo=' \
	-i -f https://github.com/mlvzk/manix/archive/master.tar.gz
```

If you're a trusted user or you don't wanna use the cachix cache you can run it without sudo.

### Nix with flakes enabled

``` sh
$ nix run 'github:mlvzk/manix' mapAttrs
```

## Kudos

The inspiration for this project came from [nix-doc](https://github.com/lf-/nix-doc)
