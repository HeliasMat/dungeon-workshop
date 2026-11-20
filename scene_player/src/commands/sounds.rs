//! The game's songs and sounds by their words, for films: `songs`, `sounds`.

use std::sync::Arc;

use ::film::Context;
use ::film::check::closest;
use ::film::words::plain;
use rules::songs::{NOISES, SONGS};

/// `songs [word]`: every song, or those the word is in (its word, its title,
/// one of its moods: `sad`, `battle`, `sea`...); `ambient`: the sounds
/// that last, for `ambient <name>`.
pub fn songs(words: &[&String], context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let library = shell::music::Library::open(data);
    let asked: Vec<String> = words.iter().map(|word| plain(word)).collect();
    let fits = |song: &&rules::songs::Song| {
        let said = format!("{} {} {}", song.word, plain(song.title), song.moods);
        asked.iter().all(|word| said.contains(word.as_str()))
    };
    let found: Vec<_> = SONGS.iter().filter(fits).collect();
    if found.is_empty() {
        let moods: Vec<&str> = SONGS.iter().flat_map(|song| song.moods.split(' ')).collect();
        let close = closest(&asked.join(" "), moods.iter().copied().chain(SONGS.iter().map(|song| song.word)), 5, 0.6);
        return Err(format!("no song for \"{}\".{}", asked.join(" "), if close.is_empty() { String::new() } else { format!(" Near: {}.", close.join(", ")) }));
    }
    for song in &found {
        // (Not in the extracted game: said, not played.)
        let missing = library.as_ref().is_some_and(|library| !library.songs.contains_key(&song.number));
        println!("{:<24} {:>3}  {:<42} {}{}", song.word, song.number, song.title, song.moods, if missing { "  (missing)" } else { "" });
    }
    println!("{} song{}: \"music <word>\"; the ambient ones, \"ambient <word>\"", found.len(), if found.len() > 1 { "s" } else { "" });
    Ok(())
}

/// `sounds [word]`: the sounds the scenes name, or those the word is in (their
/// word, when the game plays them).
pub fn noises(words: &[&String], context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let asked: Vec<String> = words.iter().map(|word| plain(word)).collect();
    let found: Vec<_> =
        NOISES.iter().filter(|noise| asked.iter().all(|word| format!("{} {}", noise.word, plain(noise.when)).contains(word.as_str()))).collect();
    // (Ours, by their file's name: a game's sound of the same name is heard instead.)
    let tables = context.tables();
    let own: Vec<&String> = tables.own_sounds.iter().filter(|name| asked.iter().all(|word| format!("{name} own ours").contains(word.as_str()))).collect();
    for name in &own {
        let seconds = shell::sound::own_length(data, &tables.project, name).map_or("?".to_string(), |frames| format!("{:.1} s", frames / 60.0));
        let hidden = if NOISES.iter().any(|noise| noise.word == name.as_str()) { " (hidden by the game's sound of the same name)" } else { "" };
        println!("{name:<14} {seconds:>7}  ours: {}/{name}.wav{hidden}", ::film::tables::OWN_SOUNDS);
    }
    if found.is_empty() && !own.is_empty() {
        println!("{} sound{} of ours: \"sound <word>\", \"ambient <word>\", \"music <word>\"", own.len(), if own.len() > 1 { "s" } else { "" });
        return Ok(());
    }
    if found.is_empty() {
        let close = closest(&asked.join(" "), NOISES.iter().map(|noise| noise.word).chain(tables.own_sounds.iter().map(String::as_str)), 5, 0.6);
        return Err(format!("no sound for \"{}\".{}", asked.join(" "), if close.is_empty() { String::new() } else { format!(" Near: {}.", close.join(", ")) }));
    }
    for noise in &found {
        println!("{:<14} {:>5}  {}", noise.word, noise.number, noise.when);
    }
    println!("{} sound{}: \"sound <word>\"; \"scene_player listen <word>\" to hear it", found.len(), if found.len() > 1 { "s" } else { "" });
    Ok(())
}

/// `listen <word | number>`: a song, a sound or a short tune of the game, by
/// its word or number (a number: a song, a sound past 1000), written to
/// `target/sounds/<word>.wav` to be heard.
pub fn listen(asked: &str, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    const SECONDS: f32 = 30.0;
    let library = shell::music::Library::open(data).ok_or("no data/rom/sound.bin: extractor <rom> first")?;
    let word = plain(asked);
    let song = SONGS.iter().find(|song| song.word == word).map(|song| song.number);
    let noise = NOISES.iter().find(|noise| noise.word == word).map(|noise| noise.number);
    let number: Option<u16> = word.parse().ok();
    let pcm = match (song, noise, number) {
        (Some(song), _, _) => library.music(song, SECONDS),
        (_, Some(noise), _) => library.sound(noise, SECONDS),
        (_, _, Some(number)) if number >= 1000 => library.sound(number, SECONDS),
        (_, _, Some(number)) => library.music(number, SECONDS),
        _ => {
            let words = SONGS.iter().map(|song| song.word).chain(NOISES.iter().map(|noise| noise.word));
            let close = closest(&word, words, 5, 0.6);
            return Err(format!(
                "neither song nor sound \"{word}\".{}",
                if close.is_empty() { String::new() } else { format!(" Near: {}.", close.join(", ")) }
            ));
        }
    };
    let pcm = pcm.ok_or(format!("\"{word}\" is not in the extracted game"))?;
    let folder = context.root().join("target/sounds");
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let out = folder.join(format!("{word}.wav"));
    std::fs::write(&out, shell::audio::wav_bytes(&pcm)).map_err(|error| error.to_string())?;
    println!("{} ({:.1} s)", out.display(), pcm.len() as f32 / shell::audio::OUTPUT_RATE as f32);
    Ok(())
}
