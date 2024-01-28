use comments_docsource::CommentDocumentation;
use options_docsource::{
    OptionDocumentation,
    OptionsDatabaseType,
};
use rayon::iter::{
    IntoParallelRefIterator,
    ParallelIterator,
};
use std::path::PathBuf;
use thiserror::Error;
use xml_docsource::XmlFuncDocumentation;

pub mod comments_docsource;
pub mod nixpkgs_tree_docsource;
pub mod options_docsource;
pub mod xml_docsource;

pub trait Cache
where
    Self: Sized + DocSource + serde::Serialize,
{
    /// Deserializes content to Self
    fn load<'a>(content: &'a [u8]) -> Result<Self, Errors>
    where
        Self: serde::Deserialize<'a>,
    {
        Ok(bincode::deserialize(content)?)
    }
    /// Saves self to a file, serialized with bincode
    fn save(&self, filename: &PathBuf) -> Result<(), Errors> {
        let x = bincode::serialize(self)?;
        std::fs::write(filename, x)?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum Errors {
    #[error("IO Error for file {}: {}", .filename, .err)]
    FileIo {
        filename: String,
        err: std::io::Error,
    },
    #[error("Failed to perform IO on a cache file")]
    CacheFileIo(#[from] std::io::Error),
    #[error("Failed to serialize/deserialize cache(bincode)")]
    Bincode(#[from] bincode::Error),
    #[error("Failed to serialize/deserialize cache(serde_json)")]
    SerdeJson(#[from] serde_json::Error),
    #[error("XML parsing error for file {}: {}", .filename, .err)]
    XmlParse {
        filename: String,
        err: roxmltree::Error,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum DocEntry {
    OptionDoc(OptionsDatabaseType, OptionDocumentation),
    CommentDoc(CommentDocumentation),
    XmlFuncDoc(XmlFuncDocumentation),
    NixpkgsTreeDoc(String),
}

impl DocEntry {
    pub fn name(&self) -> String {
        match self {
            DocEntry::OptionDoc(_, x) => x.name(),
            DocEntry::CommentDoc(x) => x.name(),
            DocEntry::XmlFuncDoc(x) => x.name(),
            DocEntry::NixpkgsTreeDoc(x) => x.clone(),
        }
    }
    pub fn pretty_printed(&self) -> String {
        match self {
            DocEntry::OptionDoc(_, x) => x.pretty_printed(),
            DocEntry::CommentDoc(x) => x.pretty_printed(),
            DocEntry::XmlFuncDoc(x) => x.pretty_printed(),
            DocEntry::NixpkgsTreeDoc(x) => x.clone(),
        }
    }
    pub fn source(&self) -> &str {
        match self {
            DocEntry::OptionDoc(typ, _) => match typ {
                OptionsDatabaseType::NixOS => "NixOS Options",
                OptionsDatabaseType::NixDarwin => "NixDarwin Options",
                OptionsDatabaseType::HomeManager => "HomeManager Options",
            },
            DocEntry::CommentDoc(_) => "Nixpkgs Comments",
            DocEntry::XmlFuncDoc(_) => "Nixpkgs Documentation",
            DocEntry::NixpkgsTreeDoc(_) => "Nixpkgs Tree",
        }
    }
}

pub trait DocSource {
    fn all_keys(&self) -> Vec<&str>;
    fn search(&self, query: &Lowercase) -> Vec<DocEntry>;
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry>;

    /// Updates the cache, returns true if anything changed
    fn update(&mut self) -> Result<bool, Errors>;
}

#[derive(Default)]
pub struct AggregateDocSource {
    sources: Vec<Box<dyn DocSource + Sync>>,
}

impl AggregateDocSource {
    pub fn add_source(&mut self, source: Box<dyn DocSource + Sync>) {
        self.sources.push(source)
    }
}

impl DocSource for AggregateDocSource {
    fn all_keys(&self) -> Vec<&str> {
        self.sources
            .par_iter()
            .flat_map(|source| source.all_keys())
            .collect()
    }
    fn search(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.sources
            .par_iter()
            .flat_map(|source| source.search(query))
            .collect()
    }
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.sources
            .par_iter()
            .flat_map(|source| source.search_liberal(query))
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        unimplemented!();
    }
}

pub struct Lowercase<'a>(pub &'a [u8]);

pub(crate) fn starts_with_insensitive_ascii(s: &[u8], prefix: &Lowercase) -> bool {
    let prefix = prefix.0;

    if s.len() < prefix.len() {
        return false;
    }

    for (i, b) in prefix.iter().enumerate() {
        // this is safe because of the earlier if check
        if unsafe { s.get_unchecked(i) }.to_ascii_lowercase() != *b {
            return false;
        }
    }

    true
}

pub(crate) fn contains_insensitive_ascii(s: &[u8], inner: &Lowercase) -> bool {
    let inner = inner.0;

    if s.len() < inner.len() {
        return false;
    }

    'outer: for i in 0..(s.len() - inner.len() + 1) {
        let target = &s[i..i + inner.len()];
        for (y, b) in target.iter().enumerate() {
            if *unsafe { inner.get_unchecked(y) } != b.to_ascii_lowercase() {
                continue 'outer;
            }
        }
        return true;
    }

    false
}

#[test]
fn test_starts_with_insensitive_ascii() {
    assert!(starts_with_insensitive_ascii(
        "This is a string".as_bytes(),
        &Lowercase(b"this ")
    ),);
    assert!(starts_with_insensitive_ascii(
        "abc".as_bytes(),
        &Lowercase(b"abc")
    ),);
    assert!(!starts_with_insensitive_ascii(
        "This is a string".as_bytes(),
        &Lowercase(b"x")
    ),);
    assert!(!starts_with_insensitive_ascii(
        "thi".as_bytes(),
        &Lowercase(b"this ")
    ),);
}

#[test]
fn test_contains_insensitive_ascii() {
    assert!(contains_insensitive_ascii(
        "abc".as_bytes(),
        &Lowercase(b"b")
    ),);
    assert!(contains_insensitive_ascii(
        "abc".as_bytes(),
        &Lowercase(b"abc")
    ),);
    assert!(contains_insensitive_ascii(
        "xabcx".as_bytes(),
        &Lowercase(b"abc")
    ),);
    assert!(!contains_insensitive_ascii(
        "abc".as_bytes(),
        &Lowercase(b"x")
    ),);
    assert!(!contains_insensitive_ascii(
        "abc".as_bytes(),
        &Lowercase(b"abcd")
    ),);
}
