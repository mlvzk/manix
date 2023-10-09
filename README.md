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

### Update

Manix is now available in nixpkgs. If you use the unstable channel installing is as easy as adding `manix` to your `environment.systemPackages` or `home.packages`.

### Github Releases

Since it can take some time to compile Manix, you can download statically-built executables from Github Releases.

```sh
wget https://github.com/lecoqjacob/manix/releases/latest/download/manix
chmod +x manix
mv manix ~/bin/ # or some other location in your $PATH
```

### nix-env

```sh
# If you have the unstable channel on your system
nix-env -f '<unstable>' -iA manix
# OR
nix-env -i -f https://github.com/lecoqjacob/manix/archive/master.tar.gz
# OR
nix profile install github:lecoqjacob/manix/latest
```

### Nix with flakes enabled

``` sh
$ nix run 'github:lecoqjacob/manix' mapAttrs
```

## Kudos

The inspiration for this project came from [nix-doc](https://github.com/lf-/nix-doc)
