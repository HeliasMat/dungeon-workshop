//! What a film can use, each by the name a film writes, with its size or
//! its length: `resources [kind] [--json]`. Read from where the project
//! keeps them (the film's tables, the game's songs and sounds, our own
//! files), never listed again here.

use std::sync::Arc;

use ::film::Context;
use std::path::{Path, PathBuf};

use rules::songs::{NOISES, SONGS};

use super::json::{self, Value, json};

/// The sorts, in the order they are told.
pub const SORTS: [&str; 10] = ["images", "items", "game_props", "sounds", "songs", "fonts", "pokemon", "variants", "places", "decors"];

/// Where a picture of ours is (`library/images`, `data/hand`).
fn drawing_path(context: &Context, file: &str) -> Option<PathBuf> {
    [context.library.images(), Path::new(&context.data).join("hand")].iter().map(|folder| folder.join(file)).find(|path| path.is_file())
}

fn relative(context: &Context, path: &Path) -> String {
    path.strip_prefix(context.root()).unwrap_or(path).display().to_string().replace('\\', "/")
}

/// The resources of a sort, each as a program reads it.
fn of_sort(sort: &str, context: &Context, measured: bool) -> Vec<Value> {
    let data = context.data.as_str();
    let known = context.tables();
    match sort {
        "images" => known
            .drawings
            .iter()
            .map(|(name, (file, cells))| {
                let path = drawing_path(context, file);
                let size = path.as_ref().and_then(|path| image::image_dimensions(path).ok());
                json!({
                    "name": name,
                    "file": path.as_deref().map(|path| relative(context, path)),
                    "size": size.map(|(wide, high)| json!([wide, high])),
                    "images": cells,
                })
            })
            .collect(),
        "items" => known.items.iter().map(|(name, row)| json!({"name": name, "number": row.0.parse::<u32>().ok(), "kind": "game item"})).collect(),
        "game_props" => known
            .props
            .iter()
            .map(|(name, object)| json!({"name": name, "number": object, "images": known.prop_frames.get(object), "kind": "game prop"}))
            .collect(),
        "sounds" => {
            let lengths = shell::sound::SoundLengths::new(data);
            let lengths = |id| lengths.of(id);
            let game = NOISES.iter().map(|noise| {
                let seconds = measured.then(|| lengths(noise.number)).flatten().map(|frames| json::seconds(frames as usize));
                json!({"name": noise.word, "number": noise.number, "when": noise.when, "duration_s": seconds, "own": false})
            });
            let tables = context.tables();
            let own = tables.own_sounds.iter().map(|name| {
                let seconds = shell::sound::own_length(data, &tables.project, name).map(|frames| json::seconds(frames as usize));
                json!({"name": name, "file": format!("{}/{name}.wav", ::film::tables::OWN_SOUNDS), "duration_s": seconds, "own": true})
            });
            game.chain(own).collect()
        }
        "songs" => SONGS.iter().map(|song| json!({"name": song.word, "number": song.number, "title": song.title, "moods": song.moods})).collect(),
        "fonts" => vec![json!({"name": "game", "says": "the game's font (the only one for now)"})],
        "pokemon" => known
            .monsters
            .iter()
            .map(|(name, row)| {
                let body = known.bodies.get(&row.0);
                json!({"name": name, "number": row.0, "says": row.1, "size": body.map(|(wide, high)| json!([wide, high]))})
            })
            .collect(),
        "variants" => known
            .variants
            .iter()
            .flat_map(|(species, names)| {
                let views = &known.views;
                names.iter().map(move |name| json!({"name": name, "species": species, "views": views.get(species)}))
            })
            .collect(),
        "places" => known
            .levels
            .iter()
            .filter(|(name, _)| !name.is_empty())
            .map(|(name, level)| json!({"name": name, "level": level.level, "size": [level.size.0, level.size.1], "middle": [level.x, level.y]}))
            .collect(),
        "decors" => {
            let mut made: Vec<Value> = known
                .decors
                .iter()
                .map(|(name, (level, x, y))| json!({"name": name, "level": level, "middle": [x, y], "kind": "place of the builder"}))
                .collect();
            let folder = context.library.decors();
            let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|kind| kind == "decor"))
                .collect();
            files.sort();
            for path in files {
                let name = path.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().into_owned());
                if !known.decors.contains_key(&name) {
                    made.push(json!({"name": name, "file": relative(context, &path), "kind": ".decor file"}));
                }
            }
            made
        }
        _ => Vec::new(),
    }
}

/// Every sort (or that one), as a program reads it.
pub fn resources_value(sort: Option<&str>, context: &Context) -> Result<Value, String> {
    let sorts: Vec<&str> = match sort {
        Some(sort) if SORTS.contains(&sort) => vec![sort],
        Some(sort) => return Err(format!("no kind of resource “{sort}”: {}", SORTS.join(", "))),
        // (The game's sounds are measured only when asked for alone: it is long.)
        None => SORTS.to_vec(),
    };
    let mut told = json::Map::new();
    for sort in sorts.iter().copied() {
        told.insert(sort.to_string(), json!(of_sort(sort, context, sorts.len() == 1)));
    }
    Ok(json::answer("resources", json!({"kinds": told})))
}

/// `resources [kind] [--json]`.
pub fn resources(sort: Option<&str>, context: &Arc<Context>, as_json: bool) -> Result<(), String> {
    let value = resources_value(sort, context)?;
    if as_json {
        json::print(&value);
        return Ok(());
    }
    for (sort, listed) in value["kinds"].as_object().into_iter().flatten() {
        let listed = listed.as_array().map_or(&[][..], Vec::as_slice);
        let names: Vec<&str> = listed.iter().filter_map(|one| one["name"].as_str()).collect();
        let shown: Vec<&str> = names.iter().take(30).copied().collect();
        let more = if names.len() > shown.len() { format!(" … (+{})", names.len() - shown.len()) } else { String::new() };
        println!("{sort} ({}): {}{more}", names.len(), shown.join(", "));
    }
    println!("--json: each with its size or its length; resources <kind>: {}", SORTS.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sort_is_told_and_a_sort_not_known_is_refused() {
        let value = resources_value(Some("fonts"), &crate::commands::testing::tested("")).expect("fonts");
        assert_eq!(value["kinds"]["fonts"][0]["name"], json!("game"));
        assert!(resources_value(Some("whatever"), &crate::commands::testing::tested("")).is_err());
        let songs = resources_value(Some("songs"), &crate::commands::testing::tested("")).expect("songs");
        assert_eq!(songs["kinds"]["songs"].as_array().map(Vec::len), Some(SONGS.len()));
    }
}
