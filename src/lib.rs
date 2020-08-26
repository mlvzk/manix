use comments_docsource::CommentDocumentation;
use core::fmt;
use options_docsource::OptionDocumentation;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
    OptionDoc(OptionDocumentation),
    CommentDoc(CommentDocumentation),
    XmlFuncDoc(XmlFuncDocumentation),
    NixpkgsTreeDoc(String),
}

impl DocEntry {
    pub fn name(&self) -> String {
        match self {
            DocEntry::OptionDoc(x) => x.name(),
            DocEntry::CommentDoc(x) => x.name(),
            DocEntry::XmlFuncDoc(x) => x.name(),
            DocEntry::NixpkgsTreeDoc(x) => x.clone(),
        }
    }
    pub fn pretty_printed(&self) -> String {
        match self {
            DocEntry::OptionDoc(x) => x.pretty_printed(),
            DocEntry::CommentDoc(x) => x.pretty_printed(),
            DocEntry::XmlFuncDoc(x) => x.pretty_printed(),
            DocEntry::NixpkgsTreeDoc(x) => x.clone(),
        }
    }
}

pub trait DocSource {
    fn all_keys(&self) -> Vec<&str>;
    fn search(&self, query: &str) -> Vec<DocEntry>;
    fn search_liberal(&self, query: &str) -> Vec<DocEntry>;

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
    fn search(&self, query: &str) -> Vec<DocEntry> {
        self.sources
            .par_iter()
            .flat_map(|source| source.search(query))
            .collect()
    }
    fn search_liberal(&self, query: &str) -> Vec<DocEntry> {
        self.sources
            .par_iter()
            .flat_map(|source| source.search_liberal(query))
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        unimplemented!()
        // for r in self
        //     .sources
        //     .par_iter_mut()
        //     .map(|s: &mut Box<dyn DocSource + Sync>| (*s).update())
        //     .collect::<Vec<Result<bool, Errors>>>()
        // {
        //     if r? {
        //         return Ok(true);
        //     }
        // }
        // Ok(false)
    }
}
