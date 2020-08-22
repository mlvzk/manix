use comments_docsource::CommentDocumentation;
use core::fmt;
use enum_dispatch::enum_dispatch;
use options_docsource::OptionDocumentation;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
use thiserror::Error;
use xml_docsource::XmlFuncDocumentation;

pub mod comments_docsource;
pub mod options_docsource;
pub mod xml_docsource;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("IO Error for file {}: {}", .filename, .err)]
    FileIo {
        filename: String,
        err: std::io::Error,
    },
    #[error("Failed to perform IO on a cache file")]
    CacheFileIo(#[from] std::io::Error),
    #[error("Failed to serialize/deserialize cache")]
    Bincode(#[from] bincode::Error),
    #[error("XML parsing error for file {}: {}", .filename, .err)]
    XmlParse {
        filename: String,
        err: roxmltree::Error,
    },
}

#[enum_dispatch(DocEntryT)]
pub enum DocEntry {
    OptionDoc(OptionDocumentation),
    CommentDoc(CommentDocumentation),
    XmlFuncDoc(XmlFuncDocumentation),
}

#[enum_dispatch]
pub trait DocEntryT {
    fn name(&self) -> String;
    fn pretty_printed(&self) -> String;
}

pub trait DocSource {
    fn all_keys(&self) -> Vec<&str>;
    fn search(&self, query: &str) -> Vec<DocEntry>;
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
}
