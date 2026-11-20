//! The player's settings, kept in a file between sessions.

/// The looks bubbles over heads may have, the player's choice: drawn
/// round and white, or in the game's own small windows.
pub const BUBBLE_STYLES: [&str; 2] = ["classic", "game windows"];
/// Where the settings are kept.
pub(crate) const SETTINGS_FILE: &str = "dungeon_settings.txt";
/// The folder of saves and settings, where the game is run from.
const SAVES: &str = "saves";
/// How fast the box writes out at each text speed, times the films' pace
/// (`engine::LETTERS_A_FRAME`): the usual one is theirs.
pub const TEXT_SPEEDS: [(&str, f32); 3] = [("slow", 0.5), ("normal", 1.0), ("fast", 3.0)];
/// The types of frames the "Window" option chooses from.
pub(crate) const WINDOW_TYPES: u8 = 5;
/// The settings each menu of options lists.
pub const OPTION_ROWS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub music: bool,
    /// 0 to 10.
    pub volume: u8,
    /// How fast messages write out: slow, normal (the original's pace),
    /// fast.
    pub text_speed: u8,
    /// The type of the windows' frames ("Window": 0 to 4).
    pub window: u8,
    /// The dungeon's map is shown over the floor ("Map").
    pub map: bool,
    /// How scenes' bubbles look (see [`BUBBLE_STYLES`]).
    pub bubbles: u8,
    /// The options of the rules every dungeon is played with, on top of
    /// its own (`rules=fairy,push_allies` in the file; no menu row).
    pub rules: crate::rule_options::RuleOptions,
}

/// Where a save or the settings are kept; one left where it used to be
/// is moved in, once.
pub fn saved(file: &str) -> String {
    let path = format!("{SAVES}/{file}");
    let _ = std::fs::create_dir_all(SAVES);
    if std::path::Path::new(file).exists() && !std::path::Path::new(&path).exists() {
        let _ = std::fs::rename(file, &path);
    }
    path
}

impl Default for Options {
    fn default() -> Self {
        Self { music: true, volume: 10, text_speed: 1, window: 0, map: true, bubbles: 0, rules: Default::default() }
    }
}

impl Options {
    /// The settings kept, or the defaults.
    pub fn load() -> Self {
        Self::read(&std::fs::read_to_string(saved(SETTINGS_FILE)).unwrap_or_default())
    }

    /// The settings of a file's text (what it does not say: the defaults).
    pub fn read(text: &str) -> Self {
        let mut options = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            if key.trim() == "rules" {
                options.rules = crate::rule_options::RuleOptions::read(value).0;
                continue;
            }
            let Ok(value) = value.trim().parse::<u8>() else { continue };
            match key.trim() {
                "music" => options.music = value != 0,
                "volume" => options.volume = value.min(10),
                "text_speed" => options.text_speed = value.min(TEXT_SPEEDS.len() as u8 - 1),
                "window" => options.window = value.min(WINDOW_TYPES - 1),
                "map" => options.map = value != 0,
                "bubbles" => options.bubbles = value.min(BUBBLE_STYLES.len() as u8 - 1),
                _ => {}
            }
        }
        options
    }

    pub fn save(&self) {
        let mut text = format!(
            "music={}\nvolume={}\ntext_speed={}\nwindow={}\nmap={}\nbubbles={}\n",
            self.music as u8, self.volume, self.text_speed, self.window, self.map as u8, self.bubbles
        );
        // (Written only once set, by hand in the file.)
        if self.rules != Default::default() {
            text.push_str(&format!("rules={}\n", self.rules.words().join(",")));
        }
        if let Err(error) = std::fs::write(saved(SETTINGS_FILE), text) {
            eprintln!("{SETTINGS_FILE}: {error}");
        }
    }

    pub(crate) fn text_speed_name(&self) -> &'static str {
        TEXT_SPEEDS[self.text_speed as usize % TEXT_SPEEDS.len()].0
    }

    /// The settings as every menu of options lists them, one a row.
    pub fn rows(&self) -> [String; OPTION_ROWS] {
        [
            format!("Music: {}", if self.music { "on" } else { "off" }),
            format!("Volume: {} / 10", self.volume),
            format!("Text: {}", self.text_speed_name()),
            format!("Window: type {}", self.window + 1),
            format!("Bubbles: {}", BUBBLE_STYLES[self.bubbles as usize % BUBBLE_STYLES.len()]),
            format!("Map: {}", if self.map { "shown" } else { "hidden" }),
        ]
    }

    /// One setting (0 music, 1 volume, 2 text speed, 3 window, 4 bubbles,
    /// 5 the dungeons' map) changed one way.
    pub fn change(&mut self, row: usize, more: bool) {
        let styles = BUBBLE_STYLES.len() as u8;
        match row {
            0 => self.music = !self.music,
            5 => self.map = !self.map,
            4 => self.bubbles = (self.bubbles + if more { 1 } else { styles - 1 }) % styles,
            1 if more => self.volume = (self.volume + 1).min(10),
            1 => self.volume = self.volume.saturating_sub(1),
            3 => self.window = (self.window + if more { 1 } else { WINDOW_TYPES - 1 }) % WINDOW_TYPES,
            _ => {
                let count = TEXT_SPEEDS.len() as u8;
                self.text_speed = (self.text_speed + if more { 1 } else { count - 1 }) % count;
            }
        }
    }
}
