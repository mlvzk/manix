use crate::{
    contains_insensitive_ascii,
    starts_with_insensitive_ascii,
    Cache,
    DocEntry,
    DocSource,
    Errors,
    Lowercase,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    collections::HashMap,
    process::Command,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct NixpkgsTreeDatabase {
    keys: Vec<String>,
}

impl Default for NixpkgsTreeDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl NixpkgsTreeDatabase {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }
}

#[derive(Serialize, Deserialize)]
struct Keys(HashMap<String, Keys>);

impl From<Keys> for Vec<String> {
    fn from(val: Keys) -> Self {
        let mut res = Vec::<String>::new();
        for (mut name, keys) in val.0 {
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
    fn search(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.keys
            .iter()
            .filter(|k| starts_with_insensitive_ascii(k.as_bytes(), query))
            .map(|k| DocEntry::NixpkgsTreeDoc(k.clone()))
            .collect()
    }
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.keys
            .iter()
            .filter(|k| contains_insensitive_ascii(k.as_bytes(), query))
            .map(|k| DocEntry::NixpkgsTreeDoc(k.clone()))
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        let new_keys = gen_keys()?;
        let old = std::mem::replace(&mut self.keys, new_keys);

        Ok(old != self.keys)
    }
}
impl Cache for NixpkgsTreeDatabase {}

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
