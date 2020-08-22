use anyhow::{Context, Result};
use comments_docsource::CommentsDatabase;
use manix::*;
use options_docsource::OptionsDatabase;

fn main() -> Result<()> {
    let cache_dir =
        xdg::BaseDirectories::with_prefix("manix").context("Failed to get a cache directory")?;

    let comment_cache_path = cache_dir
        .place_cache_file("database.bin")
        .context("Failed to place cache file")?;
    let options_hm_cache_path = cache_dir
        .place_cache_file("options_hm_database.bin")
        .context("Failed to place cache file")?;
    let options_nixos_cache_path = cache_dir
        .place_cache_file("options_nixos_database.bin")
        .context("Failed to place cache file")?;

    let mut aggregate_source = AggregateDocSource::default();

    let mut comment_db = CommentsDatabase::load(&comment_cache_path)
        .map_err(|e| anyhow::anyhow!("Failed to load NixOS comments database: {}", e))?;
    if comment_db.hash_to_defs.len() == 0 {
        eprintln!("Building NixOS comments cache...");
    }
    let cache_invalid = comment_db
        .update_cache(&comment_cache_path)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to update cache")?;
    aggregate_source.add_source(Box::new(comment_db));

    if cache_invalid {
        eprintln!("Building Home Manager cache...");
        if let Ok(options_db) = options_docsource::get_hm_json_doc_path()
            .ok()
            .and_then(|path| OptionsDatabase::try_from_file(path))
            .ok_or(anyhow::anyhow!("Failed to load Home Manager options",))
        {
            let out = bincode::serialize(&options_db).context("Failed to serialize cache")?;
            std::fs::write(&options_hm_cache_path, out).context("Failed to write cache to file")?;
            aggregate_source.add_source(Box::new(options_db));
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
            Err(e) => eprintln!("{}", e),
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
            Err(e) => eprintln!("{}", e),
        }

        match std::fs::read(&options_nixos_cache_path)
            .context("Failed to read the cache file for NixOS")
        {
            Ok(cache_bin) => {
                let options_db: OptionsDatabase = bincode::deserialize(&cache_bin)
                    .context("Failed to deserialize NixOS cache")?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("{}", e),
        }
    }

    match xml_docsource::XmlFuncDocDatabase::try_load()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to load XML documentation")
    {
        Ok(db) => aggregate_source.add_source(Box::new(db)),
        Err(e) => eprintln!("{}", e),
    }

    let search_key = std::env::args()
        .skip(1)
        .next()
        .context("You need to provide a function name (e.g. manix mkderiv)")?
        .to_lowercase();

    for entry in aggregate_source.search(&search_key) {
        println!("{}", entry.pretty_printed());
    }
    Ok(())
}
