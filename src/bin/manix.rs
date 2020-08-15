use manix::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = xdg::BaseDirectories::with_prefix("manix")
        .map(|bs| bs.place_cache_file("database.bin"))
        .map_err(|_| CustomError("Couldn't find a cache directory".into()))??;

    let mut database = Database::load(&cache_path)?;

    database
        .update_cache(&cache_path)
        .map_err(|_| CustomError("Failed to update cache".into()))?;

    let search_key = std::env::args()
        .skip(1)
        .next()
        .ok_or_else(|| {
            CustomError("You need to provide a function name (e.g. manix mkderiv)".into())
        })?
        .to_lowercase();

    for matches in database.search(&search_key) {
        println!("{}", matches.pretty_printed());
    }

    Ok(())
}
