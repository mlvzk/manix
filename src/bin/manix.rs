use anyhow::{
    Context,
    Result,
};
use colored::*;
use comments_docsource::CommentsDatabase;
use lazy_static::lazy_static;
use manix::*;
use options_docsource::{
    OptionsDatabase,
    OptionsDatabaseType,
};
use std::path::PathBuf;
use structopt::{
    clap::arg_enum,
    StructOpt,
};

arg_enum! {
    #[derive(Debug, PartialEq)]
    #[allow(non_camel_case_types)]
    enum Source {
        hm_options,
        nd_options,
        nixos_options,
        nixpkgs_doc,
        nixpkgs_tree,
        nixpkgs_comments,
    }
}

lazy_static! {
    static ref SOURCE_VARIANTS: String = Source::variants().join(",");
}

#[derive(StructOpt)]
#[structopt(name = "manix")]
struct Opt {
    /// Force update cache
    #[structopt(short, long)]
    update_cache: bool,

    /// Matches entries stricly
    #[structopt(short, long)]
    strict: bool,

    /// Restrict search to chosen sources
    #[structopt(long, possible_values = &Source::variants(), default_value = &SOURCE_VARIANTS, use_delimiter = true)]
    source: Vec<Source>,

    /// Query to search for
    #[structopt(name = "QUERY")]
    query: String,
}

fn build_source_and_add<T>(
    mut source: T,
    name: &str,
    path: &PathBuf,
    aggregate: Option<&mut AggregateDocSource>,
) -> Option<()>
where
    T: 'static + DocSource + Cache + Sync,
{
    eprintln!("Building {} cache...", name);
    if let Err(e) = source
        .update()
        .with_context(|| anyhow::anyhow!("Failed to update {}", name))
    {
        eprintln!("{:?}", e);
        return None;
    }

    if let Err(e) = source
        .save(path)
        .with_context(|| format!("Failed to save {} cache", name))
    {
        eprintln!("{:?}", e);
        return None;
    }

    if let Some(aggregate) = aggregate {
        aggregate.add_source(Box::new(source));
    }
    Some(())
}

fn load_source_and_add<T>(
    load_result: Result<Result<T, Errors>, std::io::Error>,
    name: &str,
    aggregate: &mut AggregateDocSource,
    ignore_file_io_error: bool,
) -> Option<()>
where
    T: 'static + DocSource + Cache + Sync,
{
    let load_result = match load_result {
        Err(e) => {
            if !ignore_file_io_error {
                eprintln!("Failed to load {} cache file: {:?}", name, e);
            }
            return None;
        }
        Ok(r) => r,
    };

    match load_result.with_context(|| anyhow::anyhow!("Failed to load {}", name)) {
        Err(e) => {
            eprintln!("{:?}", e);
            None
        }
        Ok(source) => {
            aggregate.add_source(Box::new(source));
            Some(())
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

    let options_nd_cache_path = cache_dir
        .place_cache_file("options_nd_database.bin")
        .context("Failed to place nix-darwin options cache file")?;
    let options_nixos_cache_path = cache_dir
        .place_cache_file("options_nixos_database.bin")
        .context("Failed to place NixOS options cache file")?;
    let options_hm_cache_path = cache_dir
        .place_cache_file("options_hm_database.bin")
        .context("Failed to place home-manager options cache file")?;
    let comment_cache_path = cache_dir
        .place_cache_file("comments.bin")
        .context("Failed to place cache file")?;
    let nixpkgs_tree_cache_path = cache_dir
        .place_cache_file("nixpkgs_tree.bin")
        .context("Failed to place nixpkgs tree cache file")?;
    let nixpkgs_doc_cache_path = cache_dir
        .place_cache_file("nixpkgs_doc_database.bin")
        .context("Failed to place Nixpkgs Documentation cache file")?;

    let version = std::env!("CARGO_PKG_VERSION");
    let last_version = std::fs::read(&last_version_path)
        .map(String::from_utf8)
        .unwrap_or(Ok(version.to_string()))?;

    let should_invalidate_cache = version != last_version;

    let mut aggregate_source = AggregateDocSource::default();

    let mut comment_db = if !should_invalidate_cache && comment_cache_path.exists() {
        CommentsDatabase::load(&std::fs::read(&comment_cache_path)?)
            .map_err(|e| anyhow::anyhow!("Failed to load Nixpkgs comments database: {:?}", e))?
    } else {
        CommentsDatabase::new()
    };
    if comment_db.hash_to_defs.is_empty() {
        eprintln!("Building Nixpkgs comments cache...");
    }

    let cache_invalid = comment_db
        .update()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to update cache")?;
    comment_db.save(&comment_cache_path)?;
    if opt.source.contains(&Source::nixpkgs_comments) {
        aggregate_source.add_source(Box::new(comment_db));
    }

    if should_invalidate_cache || opt.update_cache || cache_invalid {
        if build_source_and_add(
            OptionsDatabase::new(OptionsDatabaseType::HomeManager),
            "Home Manager Options",
            &options_hm_cache_path,
            if opt.source.contains(&Source::hm_options) {
                Some(&mut aggregate_source)
            } else {
                None
            },
        )
        .is_none()
        {
            eprintln!("Tip: If you installed your home-manager through configuration.nix you can fix this error by adding the home-manager channel with this command: {}", "nix-channel --add https://github.com/rycee/home-manager/archive/master.tar.gz home-manager && nix-channel --update".bold());
        }

        if build_source_and_add(
            OptionsDatabase::new(OptionsDatabaseType::NixDarwin),
            "Nix-Darwin Options",
            &options_nd_cache_path,
            if opt.source.contains(&Source::nd_options) {
                Some(&mut aggregate_source)
            } else {
                None
            },
        )
        .is_none()
        {
            eprintln!("Tip: Ensure darwin is set in your NIX_PATH");
        }

        build_source_and_add(
            OptionsDatabase::new(OptionsDatabaseType::NixOS),
            "NixOS Options",
            &options_nixos_cache_path,
            if opt.source.contains(&Source::nixos_options) {
                Some(&mut aggregate_source)
            } else {
                None
            },
        );

        build_source_and_add(
            nixpkgs_tree_docsource::NixpkgsTreeDatabase::new(),
            "Nixpkgs Tree",
            &nixpkgs_tree_cache_path,
            if opt.source.contains(&Source::nixpkgs_tree) {
                Some(&mut aggregate_source)
            } else {
                None
            },
        );

        build_source_and_add(
            xml_docsource::XmlFuncDocDatabase::new(),
            "Nixpkgs Documentation",
            &nixpkgs_doc_cache_path,
            if opt.source.contains(&Source::nixpkgs_doc) {
                Some(&mut aggregate_source)
            } else {
                None
            },
        );

        std::fs::write(&last_version_path, version)?;
    } else {
        if opt.source.contains(&Source::nixos_options) {
            load_source_and_add(
                std::fs::read(&options_nixos_cache_path).map(|c| OptionsDatabase::load(&c)),
                "NixOS Options",
                &mut aggregate_source,
                false,
            );
        }

        if opt.source.contains(&Source::nd_options) {
            load_source_and_add(
                std::fs::read(&options_nd_cache_path).map(|c| OptionsDatabase::load(&c)),
                "Nix Darwin Options",
                &mut aggregate_source,
                true,
            );
        }

        if opt.source.contains(&Source::hm_options) {
            load_source_and_add(
                std::fs::read(&options_hm_cache_path).map(|c| OptionsDatabase::load(&c)),
                "Home Manager Options",
                &mut aggregate_source,
                true,
            );
        }

        if opt.source.contains(&Source::nixpkgs_tree) {
            load_source_and_add(
                std::fs::read(&nixpkgs_tree_cache_path)
                    .map(|c| nixpkgs_tree_docsource::NixpkgsTreeDatabase::load(&c)),
                "Nixpkgs Tree",
                &mut aggregate_source,
                false,
            );
        }

        if opt.source.contains(&Source::nixpkgs_doc) {
            load_source_and_add(
                std::fs::read(&nixpkgs_doc_cache_path)
                    .map(|c| xml_docsource::XmlFuncDocDatabase::load(&c)),
                "Nixpkgs Documentation",
                &mut aggregate_source,
                false,
            );
        }
    }

    let query_lower = opt.query.to_ascii_lowercase();
    let query = manix::Lowercase(query_lower.as_bytes());
    let entries = if opt.strict {
        aggregate_source.search(&query)
    } else {
        aggregate_source.search_liberal(&query)
    };
    let (entries, key_only_entries): (Vec<DocEntry>, Vec<DocEntry>) = entries
        .into_iter()
        .partition(|e| !matches!(e, DocEntry::NixpkgsTreeDoc(_)));

    if !key_only_entries.is_empty() {
        const SHOW_MAX_LEN: usize = 50;
        print!("{}", "Here's what I found in nixpkgs:".bold());
        for entry in key_only_entries.iter().take(SHOW_MAX_LEN) {
            print!(" {}", entry.name().white());
        }
        if key_only_entries.len() > SHOW_MAX_LEN {
            print!(" and {} more.", key_only_entries.len() - SHOW_MAX_LEN);
        }
        println!("\n");
    }

    for entry in entries {
        const LINE: &str = "────────────────────";
        println!(
            "{}\n{}\n{}",
            entry.source().white(),
            LINE.green(),
            entry.pretty_printed()
        );
    }

    Ok(())
}
