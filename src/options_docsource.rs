use crate::{
    contains_insensitive_ascii, starts_with_insensitive_ascii, Cache, DocEntry, DocSource, Errors,
    Lowercase,
};
use colored::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Command};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionDocumentation {
    #[serde(default)]
    description: String,

    #[serde(default, rename(serialize = "readOnly", deserialize = "readOnly"))]
    read_only: bool,

    #[serde(rename(serialize = "loc", deserialize = "loc"))]
    location: Vec<String>,

    #[serde(rename(serialize = "type", deserialize = "type"))]
    option_type: String,
}

impl OptionDocumentation {
    pub fn name(&self) -> String {
        self.location.join(".")
    }
    pub fn pretty_printed(&self) -> String {
        format!(
            "# {}\n{}\ntype: {}\n\n",
            self.name().blue().bold(),
            self.description,
            self.option_type
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionsDatabaseType {
    NixOS,
    HomeManager,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionsDatabase {
    pub typ: OptionsDatabaseType,
    pub options: HashMap<String, OptionDocumentation>,
}

impl OptionsDatabase {
    pub fn new(typ: OptionsDatabaseType) -> Self {
        Self {
            typ,
            options: HashMap::new(),
        }
    }
}

pub fn try_from_file(path: &PathBuf) -> Result<HashMap<String, OptionDocumentation>, Errors> {
    let options: HashMap<String, OptionDocumentation> =
        serde_json::from_slice(&std::fs::read(path)?)?;
    Ok(options)
}

impl DocSource for OptionsDatabase {
    fn all_keys(&self) -> Vec<&str> {
        self.options.keys().map(|x| x.as_ref()).collect()
    }
    fn search(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.options
            .iter()
            .filter(|(key, _)| starts_with_insensitive_ascii(key.as_bytes(), query))
            .map(|(_, d)| DocEntry::OptionDoc(self.typ, d.clone()))
            .collect()
    }
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.options
            .iter()
            .filter(|(key, _)| contains_insensitive_ascii(key.as_bytes(), query))
            .map(|(_, d)| DocEntry::OptionDoc(self.typ, d.clone()))
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        let opts = match self.typ {
            OptionsDatabaseType::NixOS => try_from_file(&get_nixos_json_doc_path()?)?,
            OptionsDatabaseType::HomeManager => try_from_file(&get_hm_json_doc_path()?)?,
        };

        let old = std::mem::replace(&mut self.options, opts);

        Ok(old.keys().eq(self.options.keys()))
    }
}

impl Cache for OptionsDatabase {}

pub fn get_hm_json_doc_path() -> Result<PathBuf, std::io::Error> {
    let base_path_output = Command::new("nix-build")
        .arg("-E")
        .arg(
            r#"{ pkgs ? import <nixpkgs> {} }:
            let
                hmargs = { pkgs = pkgs; lib = import (<home-manager/modules/lib/stdlib-extended.nix>) pkgs.lib; };
                docs = import (<home-manager/doc>) hmargs;
            in (if builtins.isFunction docs then docs hmargs else docs).options.json
        "#)
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap())?;

    Ok(PathBuf::from(base_path_output.trim_end_matches("\n"))
        .join("share/doc/home-manager/options.json"))
}

pub fn get_nixos_json_doc_path() -> Result<PathBuf, std::io::Error> {
    let base_path_output = Command::new("nix-build")
        .env("NIXPKGS_ALLOW_UNFREE", "1")
        .env("NIXPKGS_ALLOW_BROKEN", "1")
        .env("NIXPKGS_ALLOW_INSECURE", "1")
        .arg("--no-out-link")
        .arg("-E")
        .arg(r#"with import <nixpkgs> {}; let eval = import (pkgs.path + "/nixos/lib/eval-config.nix") { modules = []; }; opts = (nixosOptionsDoc { options = eval.options; }).optionsJSON; in runCommandLocal "options.json" { inherit opts; } "cp $opts/share/doc/nixos/options.json $out""#)
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap())?;

    Ok(PathBuf::from(base_path_output.trim_end_matches("\n")))
}
