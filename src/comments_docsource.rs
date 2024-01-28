use crate::{
    contains_insensitive_ascii,
    starts_with_insensitive_ascii,
    Cache,
    DocEntry,
    DocSource,
    Errors,
    Lowercase,
};
use colored::*;
use lazy_static::lazy_static;
use rayon::prelude::*;
use rnix::{
    ast::{
        AttrSet,
        Entry,
        HasEntry,
        Ident,
        Lambda,
    },
    NodeOrToken,
    Root,
    SyntaxKind,
    SyntaxNode,
    WalkEvent,
};
use rowan::ast::AstNode;
use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashMap;

use std::{
    path::PathBuf,
    process::Command,
};
lazy_static! {
    static ref NIXPKGS_PATH: PathBuf = get_nixpkgs_root();
}

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
                NodeOrToken::Token(token) => comments.push(token.text().into()),
                NodeOrToken::Node(_) => unreachable!(),
            },
            // This stuff is found as part of `the-fn = f: ...`
            // here:                           ^^^^^^^^
            SyntaxKind::NODE_ATTRPATH | SyntaxKind::TOKEN_ASSIGN => (),
            t if t.is_trivia() => (),
            _ => break,
        }
    }

    // reverse the order because the function reads bottom-up
    comments.reverse();
    Some(comments)
}

fn visit_attr_entry(entry: Entry) -> Option<CommentDocumentation> {
    let ident = Ident::cast(entry.syntax().clone())?;
    let lambda = Lambda::cast(entry.syntax().clone())?;

    let comments = find_comments(lambda.syntax().clone()).unwrap_or_default();

    Some(CommentDocumentation::new(ident.to_string(), comments))
}

fn visit_attrset(set: &AttrSet) -> Vec<CommentDocumentation> {
    set.entries()
        .flat_map(|e| visit_attr_entry(e).into_iter())
        .collect()
}

fn walk_ast(ast: Root) -> Vec<CommentDocumentation> {
    let mut res = Vec::<CommentDocumentation>::new();
    for ev in ast.expr().unwrap().syntax().preorder_with_tokens() {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentDocumentation {
    pub key: String,
    pub path: Option<PathBuf>,
    pub comments: Vec<String>,
}

impl CommentDocumentation {
    pub fn new(key: String, comments: Vec<String>) -> Self {
        Self {
            key,
            comments,
            path: None,
        }
    }
    pub fn with_path(self, path: PathBuf) -> Self {
        CommentDocumentation {
            path: Some(path),
            ..self
        }
    }
}

pub fn cleanup_comment(s: &str) -> &str {
    s.trim_start_matches('#')
        .trim_start_matches("/*")
        .trim_end_matches("*/")
}

impl CommentDocumentation {
    pub fn pretty_printed(&self) -> String {
        let heading = self.key.blue().bold();
        let path = self
            .path
            .as_ref()
            .map(|path| {
                path.strip_prefix(NIXPKGS_PATH.to_owned())
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .unwrap_or_default()
            .white();

        let comment = self
            .comments
            .iter()
            .map(|c: &String| cleanup_comment(c))
            .collect::<Vec<&str>>()
            .join("\n");

        format!("# {} ({})\n{}\n\n", heading, path, comment)
    }
    pub fn name(&self) -> String {
        self.key.to_owned()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentsDatabase {
    pub hash_to_defs: HashMap<u32, Vec<CommentDocumentation>>,
}

impl DocSource for CommentsDatabase {
    fn all_keys(&self) -> Vec<&str> {
        self.hash_to_defs
            .values()
            .flatten()
            .map(|def| def.key.as_ref())
            .collect()
    }
    fn search(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.hash_to_defs
            .values()
            .flatten()
            .filter(|d| {
                !d.comments.is_empty() && starts_with_insensitive_ascii(d.key.as_bytes(), query)
            })
            .cloned()
            .map(DocEntry::CommentDoc)
            .collect()
    }
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.hash_to_defs
            .values()
            .flatten()
            .filter(|d| {
                !d.comments.is_empty() && contains_insensitive_ascii(d.key.as_bytes(), query)
            })
            .cloned()
            .map(DocEntry::CommentDoc)
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        let files = find_nix_files(get_nixpkgs_root())
            .par_iter()
            .filter_map(|f| match std::fs::read_to_string(f.path()) {
                Ok(content) => {
                    let mut hasher = crc32fast::Hasher::new();
                    hasher.update(content.as_bytes());
                    let hash = hasher.finalize();
                    Some((hash, f.path().to_path_buf(), content))
                }
                Err(_) => {
                    eprintln!("Skipped {}", f.path().to_str()?);
                    None
                }
            })
            .collect::<Vec<(u32, PathBuf, String)>>();

        let new_defs = files
            .par_iter()
            .filter(|(hash, _, _)| !self.is_in_cache(hash))
            .map(|(hash, path, content)| {
                let ast = rnix::Root::parse(content).ok().unwrap();
                let definitions = walk_ast(ast)
                    .into_iter()
                    .map(|def| def.with_path(path.clone()))
                    .collect();
                (hash, definitions)
            })
            .collect::<Vec<(&u32, Vec<CommentDocumentation>)>>();
        if new_defs.is_empty() {
            return Ok(false);
        }

        for (hash, defs) in new_defs {
            self.add_to_cache(*hash, defs);
        }

        Ok(true)
    }
}

impl Cache for CommentsDatabase {}
impl Default for CommentsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl CommentsDatabase {
    pub fn new() -> Self {
        Self {
            hash_to_defs: HashMap::new(),
        }
    }

    fn is_in_cache(&self, hash: &u32) -> bool {
        self.hash_to_defs.contains_key(hash)
    }

    fn add_to_cache(
        &mut self,
        hash: u32,
        defs: Vec<CommentDocumentation>,
    ) -> Option<Vec<CommentDocumentation>> {
        self.hash_to_defs.insert(hash, defs)
    }
}

fn find_nix_files(path: PathBuf) -> Vec<walkdir::DirEntry> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| !e.path().to_str().unwrap().contains("test"))
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
