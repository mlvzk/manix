use comments_docsource::CommentsDatabase;
use manix::*;
use options_docsource::OptionsDatabase;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let comment_cache_path = xdg::BaseDirectories::with_prefix("manix")
        .map(|bs| bs.place_cache_file("database.bin"))
        .map_err(|_| CustomError("Couldn't find a cache directory".into()))??;

    let options_hm_cache_path = xdg::BaseDirectories::with_prefix("manix")
        .map(|bs| bs.place_cache_file("options_hm_database.bin"))
        .map_err(|_| CustomError("Couldn't find a cache directory".into()))??;
    let options_nixos_cache_path = xdg::BaseDirectories::with_prefix("manix")
        .map(|bs| bs.place_cache_file("options_nixos_database.bin"))
        .map_err(|_| CustomError("Couldn't find a cache directory".into()))??;

    let mut aggregate_source = AggregateDocSource::default();

    let cache_invalid = if let Ok(mut comment_db) = CommentsDatabase::load(&comment_cache_path) {
        let cache_invalid = comment_db.update_cache(&comment_cache_path)?;
        aggregate_source.add_source(Box::new(comment_db));
        cache_invalid
    } else {
        false
    };

    if cache_invalid {
        if let Ok(options_db) = options_docsource::get_hm_json_doc_path()
            .ok()
            .and_then(|path| OptionsDatabase::try_from_file(path))
            .ok_or(CustomError(
                "Failed to load Home Manager options".to_string(),
            ))
        {
            let out = bincode::serialize(&options_db)
                .map_err(|_| CustomError("Failed to serialize cache".into()))?;
            std::fs::write(&options_hm_cache_path, out)
                .map_err(|_| CustomError("Failed to write cache to file".into()))?;
            aggregate_source.add_source(Box::new(options_db));
        }

        match options_docsource::get_nixos_json_doc_path()
            .ok()
            .and_then(|path| OptionsDatabase::try_from_file(path))
            .ok_or(CustomError("Failed to load NixOS options".to_string()))
        {
            Ok(options_db) => {
                let out = bincode::serialize(&options_db)
                    .map_err(|_| CustomError("Failed to serialize cache".into()))?;
                std::fs::write(&options_nixos_cache_path, out)
                    .map_err(|_| CustomError("Failed to write cache to file".into()))?;
                aggregate_source.add_source(Box::new(options_db));
            }
            Err(e) => eprintln!("Failed to load NixOS options: {:#?}", e),
        }
    } else {
        if let Ok(cache_bin) = std::fs::read(&options_hm_cache_path)
            .map_err(|_| CustomError("Failed to read the cache file HM".into()))
        {
            let options_db: OptionsDatabase = bincode::deserialize(&cache_bin)
                .map_err(|_| CustomError("Failed to deserialize cache".into()))?;
            aggregate_source.add_source(Box::new(options_db));
        }

        if let Ok(cache_bin) = std::fs::read(&options_nixos_cache_path)
            .map_err(|_| CustomError("Failed to read the cache file NIXOS".into()))
        {
            let options_db: OptionsDatabase = bincode::deserialize(&cache_bin)
                .map_err(|_| CustomError("Failed to deserialize cache".into()))?;
            aggregate_source.add_source(Box::new(options_db));
        }
    }
    let search_key = std::env::args()
        .skip(1)
        .next()
        .ok_or_else(|| {
            CustomError("You need to provide a function name (e.g. manix mkderiv)".into())
        })?
        .to_lowercase();

    for entry in aggregate_source.search(&search_key) {
        println!("{}", entry.pretty_printed());
    }
    Ok(())
}
