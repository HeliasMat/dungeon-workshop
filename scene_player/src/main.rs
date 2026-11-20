//! Plays a film by itself, in a window or at once.
//!
//! ```text
//! scene_player <command> ...       see `help`: `scene_player help [command]`
//! scene_player <file> [--size <1 to 8>] [--auto] [--from <label>]
//! scene_player                     the `film.txt` beside it, or the help
//! ```
//!
//! `--data <folder>`, anywhere, reads the game's files from there (else
//! `DUNGEON_DATA`, else the root's `data/`); `DUNGEON_FILM_LANGUAGE`
//! (`fr`, `es`…) the language the films are said in. The root (`library/`,
//! `games/`…) is found once, up from where it is run or from the program,
//! and handed to every command with the data folder (`film::Context`).
//!
//! In the window, Enter reads a line, the arrows choose an answer or walk whom
//! a scene gives the player (Shift runs, Tab the next one), Space held hurries,
//! F2 lets lines go by themselves, F5 reads the file again and plays it anew,
//! Escape quits. P pauses, `.` then goes on a frame at a time; Page Up and
//! Page Down go ten seconds back and on, F3 and F4 a scene, Home to the start.
//! For whoever writes it (`preview::debug`, `DUNGEON_FILM_DEBUG`, read here and handed
//! to the window): F6 draws what
//! the scene holds over it, F7 writes lines at once, F8 takes the fades off,
//! F9 goes through a dungeon's floors, Ctrl and a digit go to that scene.

mod commands;
mod moment;
mod pack;
mod window;

/// The `film.txt` beside the program: what a packet plays.
fn beside() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let film = exe.parent()?.join("film.txt");
    film.exists().then(|| film.to_string_lossy().into_owned())
}

fn run() -> Result<(), String> {
    let mut words: Vec<String> = std::env::args().skip(1).collect();
    // (A packet, `scene_player pack`: the program beside its film and its
    // data, no library: the packet's folder is its root.)
    let packet = beside().and_then(|film| std::path::Path::new(&film).parent().map(std::path::Path::to_path_buf));
    let library = match (game::Project::found(), packet) {
        (Ok(library), _) => library,
        (Err(_), Some(packet)) => game::Project::library(packet),
        (Err(why), None) => return Err(why),
    };
    let mut data = shell::data::folder(library.root());
    if let Some(at) = words.iter().position(|word| word == "--data") {
        data = words.get(at + 1).cloned().ok_or("--data: which folder?")?;
        words.drain(at..at + 2);
    }
    let language = std::env::var("DUNGEON_FILM_LANGUAGE").unwrap_or_default();
    let debug = preview::debug::Debug::read(&std::env::var(preview::debug::VARIABLE).unwrap_or_default());
    let context = std::sync::Arc::new(film::Context::new(library, data).with_language(&language));
    let data = context;
    match words.first() {
        None => match beside() {
            Some(film) => commands::run("play", &[film], &data, &debug),
            None => commands::run("help", &[], &data, &debug),
        },
        Some(word) if word.starts_with("--") && beside().is_some() => {
            words.extend(beside());
            commands::run("play", &words, &data, &debug)
        }
        // (A file named alone is played; whatever else, a word misspelt.)
        Some(word) if !commands::known(word) && std::path::Path::new(word).is_file() => commands::run("play", &words, &data, &debug),
        Some(word) => commands::run(word, &words[1..], &data, &debug),
    }
}

fn main() {
    if let Err(problem) = run() {
        eprintln!("{problem}");
        std::process::exit(1);
    }
}
