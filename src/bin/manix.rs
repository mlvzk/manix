use anyhow::{Context, Result};
use comments_docsource::CommentsDatabase;
use manix::*;
use options_docsource::OptionsDatabase;
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

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let cache_dir =
        xdg::BaseDirectories::with_prefix("manix").context("Failed to get a cache directory")?;

    let comment_cache_path = cache_dir
        .place_cache_file("database.bin")
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

    let mut aggregate_source = AggregateDocSource::default();

    let mut comment_db = CommentsDatabase::load(&comment_cache_path)
        .map_err(|e| anyhow::anyhow!("Failed to load NixOS comments database: {:?}", e))?;
    if comment_db.hash_to_defs.len() == 0 {
        eprintln!("Building NixOS comments cache...");
    }
    let cache_invalid = comment_db
        .update_cache(&comment_cache_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to update cache")?;
    aggregate_source.add_source(Box::new(comment_db));

    if opt.update_cache || cache_invalid {
        eprintln!("Building Home Manager Options cache...");
        match options_docsource::get_hm_json_doc_path()
            .ok()
            .and_then(|path| OptionsDatabase::try_from_file(path))
            .ok_or(anyhow::anyhow!("Failed to load Home Manager Options"))
        {
            Ok(options_db) => {
                let out = bincode::serialize(&options_db).context("Failed to serialize cache")?;
                std::fs::write(&options_hm_cache_path, out)
                    .context("Failed to write cache to file")?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("{:?}", e),
        }

        eprintln!("Building NixOS Options cache...");
        match options_docsource::get_nixos_json_doc_path()
            .ok()
            .and_then(|path| OptionsDatabase::try_from_file(path))
            .context("Failed to load NixOS options")
        {
            Ok(options_db) => {
                let out =
                    bincode::serialize(&options_db).context("Failed to serialize NixOS cache")?;
                std::fs::write(&options_nixos_cache_path, out)
                    .context("Failed to write NixOS cache")?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("{:?}", e),
        }

        eprintln!("Building Nixpkgs Tree cache...");
        let mut tree = nixpkgs_tree_docsource::NixpkgsTreeDatabase::new();
        if let Err(e) = tree
            .update_cache(&nixpkgs_tree_cache_path)
            .context("Failed to update Nixpkgs Tree cache")
        {
            eprintln!("{:?}", e);
        } else {
            aggregate_source.add_source(Box::new(tree));
        }
    } else {
        match std::fs::read(&options_hm_cache_path)
            .context("Failed to read the cache file for Home Manager")
        {
            Ok(cache_bin) => {
                let options_db: OptionsDatabase = bincode::deserialize(&cache_bin)
                    .context("Failed to deserialize Home Manager cache")?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("{:?}", e),
        }

        match std::fs::read(&options_nixos_cache_path)
            .context("Failed to read the cache file for NixOS")
        {
            Ok(cache_bin) => {
                let options_db: OptionsDatabase = bincode::deserialize(&cache_bin)
                    .context("Failed to deserialize NixOS cache")?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("{:?}", e),
        }

        match nixpkgs_tree_docsource::NixpkgsTreeDatabase::load(&nixpkgs_tree_cache_path)
            .context("Failed to read the cache file for Nixpkgs Tree")
        {
            Ok(tree) => aggregate_source.add_source(Box::new(tree)),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    match xml_docsource::XmlFuncDocDatabase::try_load()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to load XML documentation")
    {
        Ok(db) => aggregate_source.add_source(Box::new(db)),
        Err(e) => eprintln!("{:?}", e),
    }

    let entries = if opt.strict {
        aggregate_source.search(&opt.query)
    } else {
        aggregate_source.search_liberal(&opt.query)
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
