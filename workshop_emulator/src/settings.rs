//! The Workshop's settings: what the user told it once and it remembers,
//! in a small text file of their own, out of the project (`key = value`
//! lines; `launch::LaunchOptions::settings` says where). The libretro core
//! the Emulator space runs, the user's copy of the game a film is put in,
//! armips. Never in the repository: these are the user's own paths.

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

impl Settings {
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

    /// One set and written out.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
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

    #[test]
    fn remarks_and_blank_lines_are_skipped() {
        assert_eq!(parse("# hello\n\n a = b = c \n"), vec![("a".to_string(), "b = c".to_string())]);
    }
}
