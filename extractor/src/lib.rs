//! Takes the story out of the ROM, once, into a file of the project's own
//! (`extractor <rom> <folder>` writes `<folder>/story.txt`): what
//! each scene shows and says, and how scenes and dungeons follow one
//! another. The game's own scene player reads that file; nothing of the
//! original's script engine is needed to play it.
//!
//! The file is lines of a word and what goes with it, tab-separated:
//! `event <number> <name> <chapter> <step>` starts one of the story's events (the
//! original's shared routines `EVENT_...`; the main story's are named
//! `EVENT_M<chapter>_<step>` and follow one another in that order); in it, `scene
//! <level> <name>` starts a scene, with `actor <entity> <x> <y>
//! <direction>` for who stands in it at first, `backdrop <level>` when
//! the picture changes, `say <entity> <face> <text>` (entity and face `-`
//! for none), `ask <answers separated by |>`, `dungeon <number>` and
//! `ground <level>` for where the story goes on. Texts keep the game's
//! markup; a line break in a text is written `\n`.
// (Each answer's way is written; the story's variables are as it has left
// them there, see `story`.)

pub mod dump;
mod people;
mod pictures;
mod record;
mod rom_data;
mod story;
mod tables;
mod textures;
mod town;

use pictures::pictures;
use rom_data::{Interface, RomData};
use story::story;
use tables::dungeons;
use textures::sounds_and_effects;
use town::town;

/// A text as a field of a line of the files.
pub(crate) fn text(text: &str) -> String {
    text.replace('\n', "\\n").replace('\t', " ")
}

fn wrong(error: impl ToString) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

/// Writes a picture as a PNG file.
fn png(path: &str, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    let wrong = |error: image::ImageError| std::io::Error::other(error.to_string());
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8).map_err(wrong)
}

/// `extractor <rom> [folder]`: the one use of the ROM. Everything the game
/// needs is read out of it once, into files of the project's own
/// (`data/rom/` by default, overwritten: what the tools add is in
/// `data/extra/`, never touched here); the game then plays from them alone.
/// The game's levels (`levels.txt`: number, name, how its map is drawn, its
/// place, its map), the objects its scenes hold (`objects.txt`: number,
/// name) and the names shown for each place (`place_names.txt`: place,
/// name), for the films; how many levels and objects.
fn levels(data: &RomData, folder: &str) -> std::io::Result<(usize, usize)> {
    let levels: Vec<String> =
        data.levels.iter().enumerate().map(|(at, level)| format!("{at}\t{}\t{}\t{}\t{}", level.name, level.map_type, level.place_id, level.map_id)).collect();
    std::fs::write(format!("{folder}/levels.txt"), levels.join("\n") + "\n")?;
    let objects: Vec<String> = data.objects.iter().enumerate().map(|(at, name)| format!("{at}\t{name}")).collect();
    std::fs::write(format!("{folder}/objects.txt"), objects.join("\n") + "\n")?;
    let places: Vec<String> =
        data.names.places.iter().enumerate().filter(|(_, name)| !name.is_empty()).map(|(at, name)| format!("{at}\t{}", name.replace('\n', " "))).collect();
    std::fs::write(format!("{folder}/place_names.txt"), places.join("\n") + "\n")?;
    Ok((levels.len(), objects.len()))
}

pub fn run(arguments: &[String]) {
    let Some(path) = arguments.first().filter(|path| !path.starts_with("--")) else {
        eprintln!("extractor <rom> [folder]: the ROM to read is missing");
        std::process::exit(2);
    };
    let folder = arguments
        .get(1)
        .filter(|folder| !folder.starts_with("--"))
        // (No folder said: the `data/` of where it is run, or `DUNGEON_DATA`; the Workshop names its own.)
        .map_or_else(|| format!("{}/rom", shell::data::folder(std::path::Path::new("."))), |folder| shell::data::rom_folder(folder));
    let folder = folder.as_str();
    let data = match RomData::open(path) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(2);
        }
    };
    std::fs::create_dir_all(folder).expect("the folder to extract into");
    // (Only what the game counts with, to try a change of its tables.)
    if arguments.iter().any(|argument| argument == "--tables-only") {
        match dungeons(&data, &Interface::of(&data), folder) {
            Ok((dungeons, floors, tilesets)) => println!("{folder}/dungeons.bin: {dungeons} dungeons, {floors} floors, {tilesets} tilesets"),
            Err(error) => eprintln!("--extract: {error}"),
        }
        if let Err(error) = levels(&data, folder) {
            eprintln!("--extract: {error}");
        }
        return;
    }
    let moments = match story(&data, folder) {
        Ok((events, scenes, said, moments)) => {
            println!("{folder}/story.txt: {events} events, {scenes} scenes, {said} lines said");
            moments
        }
        Err(error) => {
            eprintln!("--extract: {error}");
            Vec::new()
        }
    };
    // The interface (see `UiArt::pack`).
    let interface = Interface::of(&data);
    let packed = interface.ui_art.as_ref().map(|art| art.pack());
    match packed.map(|packed| std::fs::write(format!("{folder}/interface.bin"), packed)) {
        Some(Ok(())) => println!("{folder}/interface.bin: the interface's pictures, font and texts"),
        _ => eprintln!("--extract: the interface was not written"),
    }
    match dungeons(&data, &interface, folder) {
        Ok((dungeons, floors, tilesets)) => println!("{folder}/dungeons.bin: {dungeons} dungeons, {floors} floors, {tilesets} tilesets"),
        Err(error) => eprintln!("--extract: {error}"),
    }
    match town(&data, folder, &moments) {
        Ok((places, detailed)) => println!("{folder}/town.txt: {places} places, {detailed} of them at {} moments of the story", moments.len()),
        Err(error) => eprintln!("--extract: {error}"),
    }
    match levels(&data, folder) {
        Ok((levels, objects)) => println!("{folder}/levels.txt, objects.txt, place_names.txt: {levels} levels, {objects} objects"),
        Err(error) => eprintln!("--extract: {error}"),
    }
    // Each monster's national Pokedex number (`dex.txt`: monster, number),
    // for sprites made by others (see the sprites added by hand in `data/extra/`).
    let dex: Vec<String> =
        (1..data.monsters.species_count()).filter_map(|monster| Some(format!("{monster}\t{}", data.monsters.species(monster)?.dex))).collect();
    match std::fs::write(format!("{folder}/dex.txt"), dex.join("\n") + "\n") {
        Ok(()) => println!("{folder}/dex.txt: {} monsters' Pokedex numbers", dex.len()),
        Err(error) => eprintln!("--extract: {error}"),
    }
    match people::talk(&data, folder) {
        Ok((monsters, said)) => println!("{folder}/talk.txt: {monsters} monsters' talk groups, {said} dialogues"),
        Err(error) => eprintln!("--extract: {error}"),
    }
    match people::guests(&data, folder) {
        Ok((guests, dungeons)) => println!("{folder}/guests.txt: {guests} guests, {dungeons} dungeons adding them"),
        Err(error) => eprintln!("--extract: {error}"),
    }
    match sounds_and_effects(&data, folder) {
        Ok((sounds, effects)) => println!("{folder}: {sounds} songs, tunes and sounds, {effects} effects"),
        Err(error) => eprintln!("--extract: {error}"),
    }
    // (The pictures take minutes.)
    if arguments.iter().any(|argument| argument == "--no-pictures") {
        return;
    }
    match pictures(&data, folder) {
        Ok((backdrops, sprites, faces)) => println!("{folder}: {backdrops} backdrops, {sprites} monsters drawn, {faces} faces"),
        Err(error) => eprintln!("--extract: {error}"),
    }
}
