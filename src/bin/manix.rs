use anyhow::{Context, Result};
use comments_docsource::CommentsDatabase;
use manix::*;
use options_docsource::{OptionsDatabase, OptionsDatabaseType};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "manix")]
struct Opt {
    /// Force update cache
    #[structopt(short, long)]
    update_cache: bool,
    /// Matches entries stricly
    #[structopt(short, long)]
    strict: bool,
    #[structopt(name = "QUERY")]
    query: String,
}

fn build_source_and_add<T>(
    mut source: T,
    name: &str,
    path: &PathBuf,
    aggregate: &mut AggregateDocSource,
) where
    T: 'static + DocSource + Cache + Sync,
{
    eprintln!("Building {} cache...", name);
    if let Err(e) = source
        .update()
        .with_context(|| anyhow::anyhow!("Failed to update {}", name))
    {
        eprintln!("{:?}", e);
        return;
    }

    if let Err(e) = source
        .save(&path)
        .with_context(|| format!("Failed to save {} cache", name))
    {
        eprintln!("{:?}", e);
        return;
    }

    aggregate.add_source(Box::new(source));
}

fn load_source_and_add<T>(
    load_result: Result<Result<T, Errors>, std::io::Error>,
    name: &str,
    aggregate: &mut AggregateDocSource,
) where
    T: 'static + DocSource + Cache + Sync,
{
    let load_result = match load_result {
        Err(e) => {
            eprintln!("Failed to load {} cache file: {:?}", name, e);
            return;
        }
        Ok(r) => r,
    };

    match load_result.with_context(|| anyhow::anyhow!("Failed to load {}", name)) {
        Err(e) => {
            eprintln!("{:?}", e);
        }
        Ok(source) => {
            aggregate.add_source(Box::new(source));
        }
    }
}

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let cache_dir =
        xdg::BaseDirectories::with_prefix("manix").context("Failed to get a cache directory")?;

    let last_version_path = cache_dir
        .place_cache_file("last_version.txt")
        .context("Failed to place last version file")?;

    let comment_cache_path = cache_dir
        .place_cache_file("comments.bin")
        .context("Failed to place cache file")?;
    let nixpkgs_tree_cache_path = cache_dir
        .place_cache_file("nixpkgs_tree.bin")
        .context("Failed to place nixpkgs tree cache file")?;
    let options_hm_cache_path = cache_dir
        .place_cache_file("options_hm_database.bin")
        .context("Failed to place home-manager options cache file")?;
    let options_nixos_cache_path = cache_dir
        .place_cache_file("options_nixos_database.bin")
        .context("Failed to place NixOS options cache file")?;
    let nixpkgs_doc_cache_path = cache_dir
        .place_cache_file("nixpkgs_doc_database.bin")
        .context("Failed to place Nixpkgs Documentation cache file")?;

    let version = std::env!("CARGO_PKG_VERSION");
    let last_version = std::fs::read(&last_version_path)
        .map(|c| String::from_utf8(c))
        .unwrap_or(Ok(version.to_string()))?;

    let should_invalidate_cache = version != last_version;

    let mut aggregate_source = AggregateDocSource::default();

    let mut comment_db = if !should_invalidate_cache && comment_cache_path.exists() {
        CommentsDatabase::load(&std::fs::read(&comment_cache_path)?)
            .map_err(|e| anyhow::anyhow!("Failed to load NixOS comments database: {:?}", e))?
    } else {
        CommentsDatabase::new()
    };
    if comment_db.hash_to_defs.len() == 0 {
        eprintln!("Building NixOS comments cache...");
    }
    let cache_invalid = comment_db
        .update()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to update cache")?;
    comment_db.save(&comment_cache_path)?;
    aggregate_source.add_source(Box::new(comment_db));

    if should_invalidate_cache || opt.update_cache || cache_invalid {
        build_source_and_add(
            OptionsDatabase::new(OptionsDatabaseType::HomeManager),
            "Home Manager Options",
            &options_hm_cache_path,
            &mut aggregate_source,
        );

        build_source_and_add(
            OptionsDatabase::new(OptionsDatabaseType::NixOS),
            "NixOS Options",
            &options_nixos_cache_path,
            &mut aggregate_source,
        );

        build_source_and_add(
            nixpkgs_tree_docsource::NixpkgsTreeDatabase::new(),
            "Nixpkgs Tree",
            &nixpkgs_tree_cache_path,
            &mut aggregate_source,
        );

        build_source_and_add(
            xml_docsource::XmlFuncDocDatabase::new(),
            "Nixpkgs Documentation",
            &nixpkgs_doc_cache_path,
            &mut aggregate_source,
        );

        std::fs::write(&last_version_path, version)?;
    } else {
        load_source_and_add(
            std::fs::read(&options_hm_cache_path).map(|c| OptionsDatabase::load(&c)),
            "Home Manager Options",
            &mut aggregate_source,
        );

        load_source_and_add(
            std::fs::read(&options_nixos_cache_path).map(|c| OptionsDatabase::load(&c)),
            "NixOS Options",
            &mut aggregate_source,
        );

        load_source_and_add(
            std::fs::read(&nixpkgs_tree_cache_path)
                .map(|c| nixpkgs_tree_docsource::NixpkgsTreeDatabase::load(&c)),
            "Nixpkgs Tree",
            &mut aggregate_source,
        );

        load_source_and_add(
            std::fs::read(&nixpkgs_doc_cache_path)
                .map(|c| xml_docsource::XmlFuncDocDatabase::load(&c)),
            "Nixpkgs Documentation",
            &mut aggregate_source,
        );
    }

    let query_lower = opt.query.to_ascii_lowercase();
    let query = manix::Lowercase(query_lower.as_bytes());
    let entries = if opt.strict {
        aggregate_source.search(&query)
    } else {
        aggregate_source.search_liberal(&query)
    };
    let (entries, key_only_entries): (Vec<DocEntry>, Vec<DocEntry>) =
        entries.into_iter().partition(|e| {
            if let DocEntry::NixpkgsTreeDoc(_) = e {
                false
            } else {
                true
            }
        });

    {
        use colored::*;

        if !key_only_entries.is_empty() {
            print!("{}", "Here's what I found in nixpkgs:".bold());
            for entry in key_only_entries {
                print!(" {}", entry.name().white());
            }
            println!("\n");
        }

        for entry in entries {
            const LINE: &str = "────────────────────";
            println!("{}\n{}", LINE.green(), entry.pretty_printed());
        }
    }

    Ok(())
}
