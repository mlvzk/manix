use crate::{CustomError, DocEntry, DocSource};
use colored::*;
use roxmltree::{self, Document};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::{DirEntry, WalkDir};

fn named(x: &roxmltree::Node, name: &str) -> bool {
    x.tag_name().name() == name
}

pub fn dostuff() {
    //let file = xml_files_in(&get_doc_path())[0].clone();
    for file in xml_files_in(&get_doc_path()) {
        let content = std::fs::read_to_string(&file).unwrap();
        let document = Document::parse(&content).unwrap();

        let function_nodes = document
            .descendants()
            .filter(|x| named(&x, "section"))
            .filter(|x| {
                x.first_element_child().map_or(false, |c| {
                    named(&c, "title")
                        && c.first_element_child()
                            .map_or(false, |f| named(&f, "function"))
                })
            });

        function_nodes.for_each(|node| {
            let function_name = node
                .first_element_child()
                .and_then(|x| x.first_element_child())
                .and_then(|n| n.text());
            let function_desc = node
                .descendants()
                .find(|x| named(x, "para"))
                .and_then(|n| n.text());
            let function_type = node
                .descendants()
                .find(|n| {
                    named(&n, "subtitle")
                        && n.first_element_child()
                            .map_or(false, |n| named(&n, "literal"))
                })
                .and_then(|n| n.first_element_child())
                .and_then(|n| n.text());

            let args: Vec<_> = node
                .descendants()
                .find(|n| named(&n, "variablelist"))
                .map(|list| {
                    list.children()
                        .filter(|n| n.is_element())
                        .filter_map(|entry| {
                            let name = entry.descendants().find(|n| named(&n, "varname"));
                            let desc = entry.descendants().find(|n| named(&n, "para"));
                            if let (Some(name), Some(desc)) =
                                (name.and_then(|x| x.text()), desc.and_then(|x| x.text()))
                            {
                                Some((name, desc))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let example = node
                .descendants()
                .find(|n| named(&n, "example"))
                .and_then(|n| n.descendants().find(|n| named(&n, "programlisting")))
                .map(|n| {
                    n.descendants()
                        .filter_map(|n| n.text())
                        .collect::<Vec<_>>()
                        .join("\n")
                });

            if let (Some(name), Some(desc)) = (function_name, function_desc) {
                if let Some(function_type) = function_type {
                    println!("# {} ({})", name.blue(), function_type.cyan());
                } else {
                    println!("# {}", name.blue());
                }

                println!("{}", desc);
                if !args.is_empty() {
                    println!("\nArguments:");
                    for (name, description) in args {
                        println!("  {}: {}", name.green(), description);
                    }
                }
                if let Some(example) = example {
                    println!("\nExample:");
                    for line in example.lines() {
                        println!("  {}", line.white());
                    }
                }
                println!("\n");
            }
        });
    }
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

fn get_doc_path() -> PathBuf {
    let doc_path = Command::new("nix-build")
        .arg("<nixpkgs/doc/doc-support/default.nix>")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap();
    PathBuf::from(doc_path.trim_end_matches("\n")).join("function-docs")
}
