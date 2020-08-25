use crate::{DocEntry, DocSource, Errors};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Command};

#[derive(Serialize, Deserialize)]
pub struct NixpkgsTreeDatabase {
    keys: Vec<String>,
}

impl NixpkgsTreeDatabase {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    pub fn load(path: &PathBuf) -> Result<NixpkgsTreeDatabase, Errors> {
        Ok(bincode::deserialize(&std::fs::read(path)?)?)
    }

    // returns true if cache changed
    pub fn update_cache(&mut self, cache_path: &PathBuf) -> Result<bool, Errors> {
        let new_keys = gen_keys()?;
        let last = std::mem::replace(&mut self.keys, new_keys);

        std::fs::write(&cache_path, bincode::serialize(&self)?)?;

        Ok(last != self.keys)
    }
}

#[derive(Serialize, Deserialize)]
struct Keys(HashMap<String, Keys>);

impl Into<Vec<String>> for Keys {
    fn into(self) -> Vec<String> {
        let mut res = Vec::<String>::new();
        for (mut name, keys) in self.0 {
            res.push(name.clone());
            name.push('.');
            for key in Into::<Vec<String>>::into(keys) {
                let mut name = name.clone();
                name.push_str(&key);
                res.push(name);
            }
        }
        res
    }
}

impl DocSource for NixpkgsTreeDatabase {
    fn all_keys(&self) -> Vec<&str> {
        self.keys.iter().map(|k| k.as_str()).collect()
    }
    fn search(&self, query: &str) -> Vec<DocEntry> {
        self.keys
            .iter()
            .filter(|k| k.to_lowercase().starts_with(&query.to_lowercase()))
            .map(|k| DocEntry::NixpkgsTreeDoc(k.clone()))
            .collect()
    }
    fn search_liberal(&self, query: &str) -> Vec<DocEntry> {
        self.keys
            .iter()
            .filter(|k| k.to_lowercase().contains(&query.to_lowercase()))
            .map(|k| DocEntry::NixpkgsTreeDoc(k.clone()))
            .collect()
    }
}

fn gen_keys() -> Result<Vec<String>, Errors> {
    const CODE: &str = r#"
let
  pkgs = import <nixpkgs> { };
  f = with builtins; v: (mapAttrs
    (name: value:
      if (tryEval value).success
        && ! (tryEval (pkgs.lib.isDerivation value)).value
        && isAttrs value
      then mapAttrs (_: _: {}) value
      else {}
    )
    v
  );
in
(f (pkgs // { pkgs = {}; lib = {}; })) // { lib = f pkgs.lib; }
    "#;

    let command = Command::new("nix-instantiate")
        .arg("--json")
        .arg("--strict")
        .arg("--eval")
        .arg("-E")
        .arg(CODE)
        .output()?;

    let keys = serde_json::from_slice::<Keys>(&command.stdout)?;

    Ok(Into::<Vec<String>>::into(keys))
}
