//! Try in a Film: from the Decor space, a scratch film in the decor open
//! (the Walk tool's Pokémon and a friend, put on its ground by the scene
//! itself, walking to each of its entrances), made on a thread, written in
//! `target/workshop/try/` (never the library), opened in the Film space and
//! played. Keep in Library copies it into the library's films.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use workshop_base::session::Session;

/// A scratch film being made: where it will be written, its text to come.
pub struct Trying {
    pub path: PathBuf,
    made: Receiver<String>,
}

/// The folder scratch films are written in.
pub fn folder(session: &Session) -> PathBuf {
    session.root().join("target").join("workshop").join("try")
}

/// Whether a film is a scratch one (in that folder).
pub fn is_scratch(session: &Session, film: &Path) -> bool {
    film.starts_with(folder(session))
}

/// The friend who goes with the Walk tool's Pokémon.
fn friend_of(word: &str) -> &'static str {
    if word == "piplup" { "pikachu" } else { "piplup" }
}

/// A scratch film begun for the decor open: written out first (the film reads it as it is on disk).
pub fn begin(session: &Session, decors: &mut workshop_decor::Decors) -> Result<Trying, String> {
    let open = decors.open.as_mut().ok_or("no decor open")?;
    if !open.saved() {
        open.save()?;
    }
    let name = open.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    // (Its name, else its file's, made a title: `wish_beach`, Wish Beach.)
    let title = titled(&workshop_decor::decor_title(&name, &open.text));
    let text = open.text.clone();
    let walker = decors.walker_word.trim().to_string();
    let first = if walker.is_empty() { "pikachu".to_string() } else { walker };
    let cast = [first.clone(), friend_of(&first).to_string()];
    let (data, library, tables) = (session.data().to_string(), session.library().clone(), session.tables());
    let (send, made) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let cast: Vec<&str> = cast.iter().map(String::as_str).collect();
        _ = send.send(workshop_home::new_film::tried(&name, &title, &text, &cast, &data, &library, tables));
    });
    let path = folder(session).join(format!("{}.film", open.path.file_stem().unwrap_or_default().to_string_lossy()));
    Ok(Trying { path, made })
}

impl Trying {
    /// Its film, once made: written out, its path given (or why it could not be).
    pub fn done(&self) -> Option<Result<PathBuf, String>> {
        let text = self.made.try_recv().ok()?;
        let written = self
            .path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&self.path, text))
            .map_err(|why| format!("{}: {why}", self.path.display()));
        Some(written.map(|()| self.path.clone()))
    }
}

/// A scratch film kept: copied into the library's films under a name of its
/// own (`try_wish_beach`, `try_wish_beach_2`…); where it went.
pub fn keep(session: &Session, film: &Path) -> Result<PathBuf, String> {
    let folder = session.library().films();
    let stem = film.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let path = (1..)
        .map(|at| folder.join(if at == 1 { format!("try_{stem}.film") } else { format!("try_{stem}_{at}.film") }))
        .find(|path| !path.exists())
        .ok_or("no name free")?;
    std::fs::create_dir_all(&folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    std::fs::copy(film, &path).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

/// A name as a title: its words apart, each with a capital (`wish_beach`: Wish Beach).
fn titled(name: &str) -> String {
    name.split(['_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map(|first| first.to_uppercase().chain(letters).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    /// A decor's name made a title.
    #[test]
    fn a_decor_name_is_made_a_title() {
        assert_eq!(super::titled("wish_beach"), "Wish Beach");
        assert_eq!(super::titled("The Village Well"), "The Village Well");
    }
}
