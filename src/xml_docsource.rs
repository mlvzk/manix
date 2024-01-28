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
use roxmltree::{
    self,
    Document,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XmlFuncDocumentation {
    name: String,
    description: String,
    fn_type: Option<String>,
    args: Vec<(String, String)>,
    example: Option<String>,
}

impl XmlFuncDocumentation {
    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn pretty_printed(&self) -> String {
        let mut output = String::new();
        if let Some(function_type) = &self.fn_type {
            output.push_str(&format!(
                "# {} ({})\n",
                self.name.blue().bold(),
                function_type.cyan()
            ));
        } else {
            output.push_str(&format!("# {}\n", self.name.blue()));
        }
        output.push_str(&format!("{}\n", self.description));
        if !self.args.is_empty() {
            output.push_str("\nArguments:\n");
            for (name, description) in &self.args {
                output.push_str(&format!("  {}: {}\n", name.green(), description));
            }
        }
        if let Some(example) = &self.example {
            output.push_str("\nExample:\n");
            for line in example.lines() {
                output.push_str(&format!("  {}\n", line.white()));
            }
        }
        output
    }

    fn from_function_section_node(node: &roxmltree::Node) -> Option<Self> {
        let name = node.first_element_child()?.first_element_child()?.text()?;
        let desc = node.descendants().find(|x| is_tag(x, "para"))?.text()?;
        let fn_type = node
            .descendants()
            .find(|n| {
                is_tag(n, "subtitle")
                    && n.first_element_child()
                        .map_or(false, |n| is_tag(&n, "literal"))
            })
            .and_then(|n| n.first_element_child())
            .and_then(|n| n.text())
            .map(|x| x.to_string());

        let args: Vec<_> = node
            .descendants()
            .find(|n| is_tag(n, "variablelist"))
            .map(|list| {
                list.children()
                    .filter(|n| n.is_element())
                    .filter_map(|entry| {
                        let name = entry.descendants().find(|n| is_tag(n, "varname"));
                        let desc = entry.descendants().find(|n| is_tag(n, "para"));
                        if let (Some(name), Some(desc)) =
                            (name.and_then(|x| x.text()), desc.and_then(|x| x.text()))
                        {
                            Some((name.to_owned(), desc.to_owned()))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let example = node
            .descendants()
            .find(|n| is_tag(n, "example"))
            .and_then(|n| n.descendants().find(|n| is_tag(n, "programlisting")))
            .map(|n| {
                n.descendants()
                    .filter_map(|n| n.text())
                    .collect::<Vec<_>>()
                    .join("")
                    .to_string()
            });
        Some(XmlFuncDocumentation {
            name: name.to_owned(),
            description: desc.to_owned(),
            fn_type,
            example,
            args,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlFuncDocDatabase {
    pub functions: HashMap<String, XmlFuncDocumentation>,
}

impl Default for XmlFuncDocDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlFuncDocDatabase {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
}

impl Cache for XmlFuncDocDatabase {}

impl DocSource for XmlFuncDocDatabase {
    fn all_keys(&self) -> Vec<&str> {
        self.functions.keys().map(|x| x.as_str()).collect()
    }
    fn search(&self, query: &Lowercase) -> Vec<crate::DocEntry> {
        self.functions
            .iter()
            .filter(|(key, _)| starts_with_insensitive_ascii(key.as_bytes(), query))
            .map(|(_, value)| DocEntry::XmlFuncDoc(value.clone()))
            .collect()
    }
    fn search_liberal(&self, query: &Lowercase) -> Vec<DocEntry> {
        self.functions
            .iter()
            .filter(|(key, _)| contains_insensitive_ascii(key.as_bytes(), query))
            .map(|(_, value)| DocEntry::XmlFuncDoc(value.clone()))
            .collect()
    }
    fn update(&mut self) -> Result<bool, Errors> {
        let doc_path = &generate_docs();
        let mut result = Vec::new();
        for file in xml_files_in(doc_path) {
            let content = std::fs::read_to_string(&file).map_err(|e| Errors::FileIo {
                err: e,
                filename: file.to_str().unwrap().to_string(),
            })?;
            let document = Document::parse(&content).map_err(|e| Errors::XmlParse {
                err: e,
                filename: file.to_str().unwrap().to_string(),
            })?;

            let mut function_entries = document
                .descendants()
                .filter(|x| is_tag(x, "section"))
                .filter(|x| {
                    x.first_element_child().map_or(false, |c| {
                        is_tag(&c, "title")
                            && c.first_element_child()
                                .map_or(false, |f| is_tag(&f, "function"))
                    })
                })
                .filter_map(|node| XmlFuncDocumentation::from_function_section_node(&node))
                .collect::<Vec<_>>();
            result.append(&mut function_entries);
        }

        let new = result.into_iter().map(|x| (x.name(), x)).collect();
        let old = std::mem::replace(&mut self.functions, new);

        Ok(!self.functions.keys().eq(old.keys()))
    }
}

fn is_tag(x: &roxmltree::Node, name: &str) -> bool {
    x.tag_name().name() == name
}

fn xml_files_in(path: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("xml"))
        .map(|x| x.path().to_path_buf())
        .collect::<Vec<PathBuf>>()
}

fn generate_docs() -> PathBuf {
    let doc_path = Command::new("nix-build")
        .arg("--no-out-link")
        .arg("<nixpkgs/doc/doc-support/default.nix>")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap();
    PathBuf::from(doc_path.trim_end_matches('\n')).join("function-docs")
}
