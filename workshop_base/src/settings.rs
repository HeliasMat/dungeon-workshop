//! The Workshop's settings: what the user told it once and it remembers,
//! in a small text file of their own, out of the project (`key = value`
//! lines; `launch::LaunchOptions::settings` says where). The libretro core
//! the Emulator space runs, the user's copy of the game a film is put in,
//! armips; whether the game-making spaces show; the panels each space shows.
//! Never in the repository: these are the user's own.

use std::path::{Path, PathBuf};

/// The settings, as read; written back whole when one changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// Where they are written (none: nowhere, kept for this run).
    pub file: Option<PathBuf>,
    lines: Vec<(String, String)>,
}

/// The keys known.
pub const CORE: &str = "libretro_core";
pub const ROM: &str = "ds_rom";
pub const ARMIPS: &str = "armips";
/// `game_spaces = on|off`: the Game, Dungeon and Emulator spaces shown.
pub const GAME_SPACES: &str = "game_spaces";
/// `panels_<space> = Script,Scene,…`: the panels a space shows (View › Add Panel).
pub const PANELS: &str = "panels_";

impl Settings {
    /// Whether the game-making spaces (Game, Dungeon, Emulator) show. Said
    /// (`game_spaces`): as said. Not said: shown, to everyone (Settings ›
    /// Show the Game-Making Spaces hides them).
    pub fn game_spaces(&self, _has_games: bool) -> bool {
        match self.get(GAME_SPACES) {
            Some(said) => matches!(said.to_lowercase().as_str(), "on" | "yes" | "true" | "1"),
            None => true,
        }
    }

    /// The game-making spaces shown or hidden, and written out.
    pub fn set_game_spaces(&mut self, on: bool) -> Result<(), String> {
        self.set(GAME_SPACES, if on { "on" } else { "off" })
    }

    /// The panels a space shows, as they were left (none said: its own).
    pub fn panels(&self, space: &str) -> Option<Vec<String>> {
        let said = self.lines.iter().find(|(key, _)| key.strip_prefix(PANELS) == Some(space)).map(|(_, value)| value.as_str())?;
        Some(said.split(',').map(str::trim).filter(|panel| !panel.is_empty()).map(str::to_string).collect())
    }

    /// The panels a space shows, written out.
    pub fn set_panels(&mut self, space: &str, panels: &[&str]) -> Result<(), String> {
        self.set(&format!("{PANELS}{space}"), &panels.join(","))
    }

    /// Those of that file (none yet: empty).
    pub fn read(file: Option<PathBuf>) -> Settings {
        let text = file.as_deref().and_then(|file| std::fs::read_to_string(file).ok()).unwrap_or_default();
        Settings { file, lines: parse(&text) }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().find(|(known, _)| known == key).map(|(_, value)| value.as_str()).filter(|value| !value.is_empty())
    }

    /// A path set, if one is.
    pub fn path(&self, key: &str) -> Option<PathBuf> {
        self.get(key).map(PathBuf::from)
    }

    /// One set and written out (over the file as it is now: what another part
    /// of the Workshop wrote there since is kept).
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        if let Some(text) = self.file.as_deref().and_then(|file| std::fs::read_to_string(file).ok()) {
            self.lines = parse(&text);
        }
        match self.lines.iter_mut().find(|(known, _)| known == key) {
            Some((_, old)) => *old = value.to_string(),
            None => self.lines.push((key.to_string(), value.to_string())),
        }
        self.write()
    }

    fn write(&self) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
        }
        let mut text = String::from("# Dungeon Workshop's settings: key = value\n");
        for (key, value) in &self.lines {
            text.push_str(&format!("{key} = {value}\n"));
        }
        std::fs::write(file, text).map_err(|why| format!("{}: {why}", file.display()))
    }

    /// The folder beside the file, for what the Workshop keeps of its own there (the emulator's saves).
    pub fn folder(&self) -> PathBuf {
        self.file.as_deref().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("target"))
    }
}

/// `key = value` lines; `#` begins a remark.
fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_read_and_written_back() {
        let file = std::env::temp_dir().join(format!("workshop_settings_{}.txt", std::process::id()));
        let mut settings = Settings::read(Some(file.clone()));
        assert_eq!(settings.get(CORE), None);
        settings.set(CORE, "C:/cores/melonds_libretro.dll").unwrap();
        settings.set(ROM, "C:/games/sky.nds").unwrap();
        let again = Settings::read(Some(file.clone()));
        assert_eq!(again.get(CORE), Some("C:/cores/melonds_libretro.dll"));
        assert_eq!(again.path(ROM), Some(PathBuf::from("C:/games/sky.nds")));
        _ = std::fs::remove_file(file);
    }

    /// The game spaces: as said; not said, shown to everyone, someone new too.
    #[test]
    fn the_game_spaces_show_unless_hidden() {
        let read = |text: &str| Settings { file: None, lines: parse(text) };
        assert!(read("").game_spaces(false), "someone new: shown");
        assert!(read("panels_film = Script,Scene\n").game_spaces(false), "the panels laid out: shown");
        assert!(!read("ds_rom = C:/sky.nds\ngame_spaces = off\n").game_spaces(true), "said: as said");
        let mut settings = read("");
        settings.set_game_spaces(false).unwrap();
        assert!(!settings.game_spaces(true));
        settings.set_game_spaces(true).unwrap();
        assert!(settings.game_spaces(false));
    }

    /// The panels of a space, as they were left.
    #[test]
    fn the_panels_are_remembered() {
        let mut settings = Settings { file: None, lines: parse("panels_film = Script, Scene,Assets\n") };
        assert_eq!(settings.panels("film"), Some(vec!["Script".to_string(), "Scene".to_string(), "Assets".to_string()]));
        assert_eq!(settings.panels("decor"), None);
        settings.set_panels("decor", &["Scene", "Library"]).unwrap();
        assert_eq!(settings.panels("decor"), Some(vec!["Scene".to_string(), "Library".to_string()]));
    }

    #[test]
    fn remarks_and_blank_lines_are_skipped() {
        assert_eq!(parse("# hello\n\n a = b = c \n"), vec![("a".to_string(), "b = c".to_string())]);
    }
}
