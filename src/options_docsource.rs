use crate::{DocEntry, DocSource, DocEntryT};
use colored::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl DocEntryT for OptionDocumentation {
    fn name(&self) -> String {
        self.location.join(".")
    }
    fn pretty_printed(&self) -> String {
        format!(
            "# {}\n{}\ntype: {}\n\n",
            self.name().blue(),
            self.description,
            self.option_type
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsDatabase {
    pub options: HashMap<String, OptionDocumentation>,
}

impl OptionsDatabase {
    pub fn try_from_file<P: AsRef<Path>>(path: P) -> Option<Self> {
        let reader = BufReader::new(File::open(path).ok()?);
        let options = serde_json::from_reader(reader).ok()?;
        Some(OptionsDatabase { options })
    }
}

impl DocSource for OptionsDatabase {
    fn all_keys(&self) -> Vec<&str> {
        self.options.keys().map(|x| x.as_ref()).collect()
    }
    fn search(&self, query: &str) -> Vec<DocEntry> {
        self.options
            .iter()
            .filter(|(key, _)| key.to_lowercase().starts_with(&query.to_lowercase()))
            .map(|(_, d)| DocEntry::OptionDoc(d.clone()))
            .collect()
    }
}

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
        .arg("--no-out-link")
        .arg("-E")
        .arg(r#"with import <nixpkgs> {}; let eval = import (pkgs.path + "/nixos/lib/eval-config.nix") { modules = []; }; opts = (nixosOptionsDoc { options = eval.options; }).optionsJSON; in runCommandLocal "options.json" { inherit opts; } "cp $opts/share/doc/nixos/options.json $out""#)
        .output()
        .map(|o| String::from_utf8(o.stdout).unwrap())?;

    Ok(PathBuf::from(base_path_output.trim_end_matches("\n")))
}
