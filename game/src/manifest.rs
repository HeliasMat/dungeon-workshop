//! A pack's manifest (`dwpack.txt`, at its root): the version of its
//! layout, the game's title, its author, its `.game`, its files. What a
//! pack is made of and how it is read back is `pack`'s; what is said when
//! a pack is wrong (`Error::Pack`) names it too.

/// The version of the pack's layout, in its manifest.
pub const FORMAT: u32 = 1;

/// The manifest's name, at the root of the pack.
pub const MANIFEST: &str = "dwpack.txt";

/// A pack's manifest, read back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub format: u32,
    pub title: String,
    pub author: String,
    /// The `.game` in the pack.
    pub game: String,
    pub files: Vec<String>,
}

impl Manifest {
    /// As it is written in the pack.
    pub fn text(&self) -> String {
        let mut text = String::from("# A game of Dungeon Workshop: its own files only, nothing of the game's data.\n");
        text.push_str(&format!("format: {}\ntitle: {}\nauthor: {}\ngame: {}\n", self.format, self.title, self.author, self.game));
        for file in &self.files {
            text.push_str(&format!("file: {file}\n"));
        }
        text
    }

    /// From its text.
    pub fn read(text: &str) -> Manifest {
        let mut manifest = Manifest::default();
        for line in text.lines().filter(|line| !line.trim_start().starts_with('#')) {
            let Some((key, value)) = line.split_once(':') else { continue };
            let value = value.trim().to_string();
            match key.trim() {
                "format" => manifest.format = value.parse().unwrap_or(0),
                "title" => manifest.title = value,
                "author" => manifest.author = value,
                "game" => manifest.game = value,
                "file" => manifest.files.push(value),
                _ => {}
            }
        }
        manifest
    }
}
