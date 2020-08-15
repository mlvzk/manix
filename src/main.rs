use colored::*;
use core::fmt;
use rayon::prelude::*;
use rnix::{
    types::{AttrSet, EntryHolder, Ident, KeyValue, Lambda, TypedNode},
    NodeOrToken, SyntaxKind, SyntaxNode, WalkEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{io::prelude::*, path::PathBuf, process::Command};

fn find_comments(node: SyntaxNode) -> Option<Vec<String>> {
    let mut node = NodeOrToken::Node(node);
    let mut comments = Vec::<String>::new();

    loop {
        loop {
            if let Some(new) = node.prev_sibling_or_token() {
                node = new;
                break;
            } else {
                node = NodeOrToken::Node(node.parent()?);
            }
        }

        match node.kind() {
            SyntaxKind::TOKEN_COMMENT => match &node {
                NodeOrToken::Token(token) => comments.push(token.text().clone().into()),
                NodeOrToken::Node(_) => unreachable!(),
            },
            // This stuff is found as part of `the-fn = f: ...`
            // here:                           ^^^^^^^^
            SyntaxKind::NODE_KEY | SyntaxKind::TOKEN_ASSIGN => (),
            t if t.is_trivia() => (),
            _ => break,
        }
    }

    // reverse the order because the function reads bottom-up
    comments.reverse();
    Some(comments)
}

fn visit_attr_entry(entry: KeyValue) -> Option<Definition> {
    let ident = Ident::cast(entry.key()?.path().nth(0)?)?.node().text();
    let lambda = Lambda::cast(entry.value()?)?;

    let comments = find_comments(lambda.node().clone()).unwrap_or_else(|| Vec::new());

    Some(Definition::new(ident.to_string(), comments))
}

fn visit_attrset(set: &AttrSet) -> Vec<Definition> {
    set.entries()
        .flat_map(|e| visit_attr_entry(e).into_iter())
        .collect()
}

fn walk_ast(ast: rnix::AST) -> Vec<Definition> {
    let mut res = Vec::<Definition>::new();
    for ev in ast.node().preorder_with_tokens() {
        match ev {
            WalkEvent::Enter(enter) => {
                if let Some(set) = enter.into_node().and_then(AttrSet::cast) {
                    res.append(&mut visit_attrset(&set));
                }
            }
            WalkEvent::Leave(_) => {}
        }
    }

    res
}

#[derive(Debug, Serialize, Deserialize)]
struct Definition {
    key: String,
    comments: Vec<String>,
}
impl Definition {
    fn new(key: String, comments: Vec<String>) -> Self {
        Self { key, comments }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Database {
    hash_to_defs: HashMap<u32, Vec<Definition>>,
}

impl Database {
    fn new() -> Self {
        Self {
            hash_to_defs: HashMap::new(),
        }
    }

    fn is_in_cache(&self, hash: &u32) -> bool {
        self.hash_to_defs.contains_key(hash)
    }

    fn add_to_cache(&mut self, hash: u32, defs: Vec<Definition>) -> Option<Vec<Definition>> {
        self.hash_to_defs.insert(hash, defs)
    }

    /// if anything was updated, bool will be true
    fn update_cache(
        &mut self,
        files: Vec<(u32, String)>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let new_defs = files
            .par_iter()
            .filter(|(hash, _)| !self.is_in_cache(hash))
            .map(|(hash, content)| {
                let ast = rnix::parse(&content);
                let definitions = walk_ast(ast);
                (hash, definitions)
            })
            .collect::<Vec<(&u32, Vec<Definition>)>>();
        if new_defs.is_empty() {
            return Ok(false);
        }

        for (hash, defs) in new_defs {
            self.add_to_cache(*hash, defs);
        }

        Ok(true)
    }
}

fn find_nix_files(path: PathBuf) -> Vec<walkdir::DirEntry> {
    walkdir::WalkDir::new(&path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("nix"))
        .collect::<Vec<walkdir::DirEntry>>()
}

fn get_nixpkgs_root() -> PathBuf {
    let channel_path = Command::new("nix-instantiate")
        .arg("--eval")
        .arg("--strict")
        .arg("-E")
        .arg("<nixpkgs>")
        .output()
        .map(|o| String::from_utf8(o.stdout));

    if let Ok(Ok(path)) = channel_path {
        PathBuf::from(path.trim_end())
    } else {
        PathBuf::from(".")
    }
}

struct CustomError(String);
impl fmt::Debug for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CustomError {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = PathBuf::from("cache.bin");

    let mut database = if cache_path.exists() {
        let cache_bin = std::fs::read(&cache_path)
            .map_err(|_| CustomError("Failed to read the cache file".into()))?;
        bincode::deserialize(&cache_bin)
            .map_err(|_| CustomError("Failed to deserialize cache".into()))?
    } else {
        Database::new()
    };

    let nixpkgs_root = get_nixpkgs_root();

    let contents = find_nix_files(nixpkgs_root)
        .par_iter()
        .map(|f| {
            let content = std::fs::read_to_string(f.path()).unwrap();
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(content.as_bytes());
            let hash = hasher.finalize();
            (hash, content)
        })
        .collect::<Vec<(u32, String)>>();

    let cache_changed = database
        .update_cache(contents)
        .map_err(|_| CustomError("Failed to update cache".into()))?;

    let search_key = std::env::args()
        .skip(1)
        .next()
        .ok_or_else(|| {
            CustomError("You need to provide a function name (e.g. manix mkderiv)".into())
        })?
        .to_lowercase();

    for matches in database
        .hash_to_defs
        .values()
        .flatten()
        .filter(|d| d.comments.len() > 0 && d.key.to_lowercase().starts_with(&search_key))
    {
        let comment = matches
            .comments
            .iter()
            .map(|c: &String| {
                c.trim_start_matches("#")
                    .trim_start_matches("/*")
                    .trim_end_matches("*/")
                    .to_owned()
            })
            .collect::<Vec<String>>()
            .join("\n");

        println!("# {}\n{}\n\n", matches.key.blue(), comment);
    }

    if cache_changed {
        let out = bincode::serialize(&database)
            .map_err(|_| CustomError("Failed to serialize cache".into()))?;
        std::fs::write(&cache_path, out)
            .map_err(|_| CustomError("Failed to write cache to file".into()))?;
    }

    Ok(())
}
