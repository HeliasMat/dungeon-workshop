//! What each word of the language becomes on the DS (`scene_player
//! export-ds`): Explorers of Sky's own operations, with less than the film
//! shows, not written yet, or none. Declared with each word (`OnDs`), read
//! by the export, the language's DS mode (`for ds`) and the Workshop's Wiki
//! and completion.
//! A film `for ds` is told what the DS can't play of it as it is compiled
//! (`compile/for_ds.rs`).

/// A film written `for ds`: `boot`, played as a new game starts (`for
/// ds`), else in place of a scene of a game going on (`for ds scene`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub boot: bool,
    /// The line that says so.
    pub line: usize,
}

/// The header line that writes a film for the DS.
pub const HEADER: &str = "for ds";

/// Whether a film's text is written for the DS (a line `for ds` before its
/// first scene).
pub fn written_for_ds(text: &str) -> bool {
    text.lines().map(str::trim).take_while(|line| !line.starts_with("scene")).any(|line| line == HEADER || line.starts_with("for ds "))
}

/// The film's text with its line `for ds` (`on`) or without it: written
/// after its title, if it has one, else first (the Workshop's DS Mode).
pub fn with_ds(text: &str, on: bool) -> String {
    if written_for_ds(text) == on {
        return text.to_string();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    let head = lines.iter().position(|line| line.trim_start().starts_with("scene")).unwrap_or(lines.len());
    if on {
        let after = lines[..head].iter().position(|line| line.trim_start().starts_with("title:")).map_or(0, |at| at + 1);
        lines.insert(after, HEADER);
    } else if let Some(at) = lines[..head].iter().position(|line| line.trim() == HEADER || line.trim().starts_with("for ds ")) {
        lines.remove(at);
    }
    lines.join("\n")
}

/// Pokémon Sky shows at once in a scene (`export-ds` makes room as
/// `Writer::fit` says): the game's 24, less the hero's,
/// the partner's and a third party member's, kept for them.
pub const AT_ONCE: usize = 21;

/// The DS's screen, pixels, and how far round someone's feet its sprite
/// shows (sideways, above, below).
pub const SCREEN: (f32, f32) = (256.0, 192.0);
pub const SPRITE: (f32, f32, f32) = (16.0, 32.0, 8.0);

/// Whether those points are wider or taller than the screen shows, their
/// sprites with them: Sky has no zoom to step back.
pub fn wider_than_the_screen(points: &[(f32, f32)]) -> bool {
    if points.is_empty() {
        return false;
    }
    let (left, right) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.0), hi.max(point.0)));
    let (top, bottom) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.1), hi.max(point.1)));
    right - left + 2.0 * SPRITE.0 > SCREEN.0 || bottom - top + SPRITE.1 + SPRITE.2 > SCREEN.1
}

/// How a word of the language exports to the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Export {
    /// Written as the game's own operations, as the film plays it.
    Direct,
    /// Written, with less than the film shows (what: `OnDs::lost`).
    Close,
    /// The game can do it; the exporter does not write it yet.
    Work,
    /// The game has no operation for it: it needs a patch of the game's code.
    None,
}

/// How a word stands on the DS, declared with the word (`Word::ds_direct`,
/// `ds_close`, `ds_work`, `ds_none`, `ds_fix`; `vocabulary::OTHERS`): how it
/// exports, the game's operations it becomes (or why none), what it loses
/// (`Close` only), how to write it another way (one that does not export).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnDs {
    pub export: Export,
    pub how: &'static str,
    pub lost: Option<&'static str>,
    pub fix: Option<&'static str>,
}

impl OnDs {
    pub const fn direct(how: &'static str) -> OnDs {
        OnDs { export: Export::Direct, how, lost: None, fix: None }
    }

    pub const fn close(how: &'static str, lost: &'static str) -> OnDs {
        OnDs { export: Export::Close, how, lost: Some(lost), fix: None }
    }

    pub const fn work(how: &'static str) -> OnDs {
        OnDs { export: Export::Work, how, lost: None, fix: None }
    }

    pub const fn none(how: &'static str) -> OnDs {
        OnDs { export: Export::None, how, lost: None, fix: None }
    }

    /// How to write it another way.
    pub const fn instead(self, fix: &'static str) -> OnDs {
        OnDs { fix: Some(fix), ..self }
    }
}

/// What a scene whose place is a decor of the Workshop's loses on the DS
/// (`compile::for_ds`).
pub const DECOR_LOSS: &str = "a map of its own, still: its animated tiles and changing colours as they first are, a tile of too many colours brought to its palette's nearest; the top screen black";

impl Export {
    /// Whether it is written in the export (with or without a loss).
    pub fn exports(self) -> bool {
        matches!(self, Export::Direct | Export::Close)
    }

    /// Its badge, as the Wiki shows it: `DS direct`, `DS with loss`, `not on DS`.
    pub fn badge(self) -> &'static str {
        match self {
            Export::Direct => "DS direct",
            Export::Close => "DS with loss",
            Export::Work => "not on DS yet",
            Export::None => "not on DS",
        }
    }

    /// Its word in `export-ds --table`.
    pub fn kind(self) -> &'static str {
        match self {
            Export::Direct => "direct",
            Export::Close => "close",
            Export::Work => "work",
            Export::None => "none",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ds_mode_puts_its_line_after_the_title_and_takes_it_off() {
        let film = "title: The Bet\nscene: beach krabby\nkrabby: Hi.\n";
        let on = with_ds(film, true);
        assert_eq!(on, "title: The Bet\nfor ds\nscene: beach krabby\nkrabby: Hi.\n");
        assert!(written_for_ds(&on));
        assert_eq!(with_ds(&on, true), on);
        assert_eq!(with_ds(&on, false), film);
        assert_eq!(with_ds("scene: beach krabby\n", true), "for ds\nscene: beach krabby\n");
        assert_eq!(with_ds("for ds scene\nscene: beach krabby\n", false), "scene: beach krabby\n");
    }
}
