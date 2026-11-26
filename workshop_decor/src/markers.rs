//! The marker tools: a resident, a spot, an entrance, an exit or a zone
//! laid by a click on the Scene, dragged to move it, its essentials filled
//! in a small window beside it (a resident's Pokémon and first line, an
//! entrance's name, where an exit leads, a spot's or a zone's name). What
//! each writes lives here, apart from the Scene, so that the tests read it
//! as the decors' tool does.

use eframe::egui;

use crate::edit::{self, Kind};

/// What a marker tool lays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Marker {
    Resident,
    Spot,
    Entrance,
    Exit,
    Zone,
}

impl Marker {
    pub const ALL: [Marker; 5] = [Marker::Resident, Marker::Spot, Marker::Entrance, Marker::Exit, Marker::Zone];

    /// The kind of thing it lays.
    pub fn kind(self) -> Kind {
        match self {
            Marker::Resident => Kind::Lives,
            Marker::Spot => Kind::Spot,
            Marker::Entrance => Kind::Entrance,
            Marker::Exit => Kind::Exit,
            Marker::Zone => Kind::Zone,
        }
    }

    /// Its name on the toolbar.
    pub fn name(self) -> &'static str {
        match self {
            Marker::Resident => "Resident",
            Marker::Spot => "Spot",
            Marker::Entrance => "Entrance",
            Marker::Exit => "Exit",
            Marker::Zone => "Zone",
        }
    }

    /// Its word (`WORKSHOP_TOOL=resident`).
    pub fn word(self) -> &'static str {
        match self {
            Marker::Resident => "resident",
            Marker::Spot => "spot",
            Marker::Entrance => "entrance",
            Marker::Exit => "exit",
            Marker::Zone => "zone",
        }
    }

    pub fn of_word(word: &str) -> Option<Marker> {
        Marker::ALL.into_iter().find(|marker| marker.word() == word)
    }

    /// Its key, and the key itself.
    pub fn key(self) -> (&'static str, egui::Key) {
        match self {
            Marker::Resident => ("K", egui::Key::K),
            Marker::Spot => ("T", egui::Key::T),
            Marker::Entrance => ("E", egui::Key::E),
            Marker::Exit => ("X", egui::Key::X),
            Marker::Zone => ("Z", egui::Key::Z),
        }
    }

    /// What it does, said on the toolbar.
    pub fn about(self) -> &'static str {
        match self {
            Marker::Resident => "someone who lives there: click to place, drag to move",
            Marker::Spot => "a spot for the films (`pikachu put well`): click to place, drag to move",
            Marker::Entrance => "where the films come in (`arrives … by:north_gate`): click to place, drag to move",
            Marker::Exit => "a zone that leads to a game place or another decor: click to place, drag to move",
            Marker::Zone => "a named zone: walking in plays the place's `on zone <name>`: click to place, drag to move",
        }
    }
}

/// A Pokémon sought for a resident, in the window beside it: what is typed,
/// the search asked of the Pokémon catalog (the Workshop answers it, the
/// Resources' catalog being no part of this crate), what it found (each its
/// word and its name).
#[derive(Default)]
pub struct Species {
    pub sought: String,
    pub asked: Option<String>,
    pub found: Vec<(String, String)>,
}

/// A name of that kind no thing of the decor has yet: `spot1`, `zone2`.
pub fn new_name(text: &str, kind: Kind, base: &str) -> String {
    let names: Vec<String> = edit::items(text).into_iter().filter(|one| one.kind == kind).map(|one| one.name).collect();
    (1..).map(|at| format!("{base}{at}")).find(|name| !names.contains(name)).unwrap_or_default()
}

/// The line a marker tool lays at that point (a resident: of that Pokémon,
/// facing down): feet, a spot, an entrance at the point; an exit's or a
/// zone's middle there.
pub fn line_at(text: &str, marker: Marker, [x, y]: [f32; 2], resident: &str) -> String {
    let [x, y] = [x.round() as i64, y.round() as i64];
    match marker {
        Marker::Resident => format!("resident {} {x} {y} down", if resident.trim().is_empty() { "pikachu" } else { resident.trim() }),
        Marker::Spot => format!("spot {} {x} {y}", new_name(text, Kind::Spot, "spot")),
        Marker::Entrance => format!("entrance {x} {y}"),
        Marker::Exit => format!("exit {} {} 32 16 to 224", x - 16, y - 8),
        Marker::Zone => format!("zone {} {} {} 32 32", new_name(text, Kind::Zone, "zone"), x - 16, y - 16),
    }
}

/// The decor with that marker laid there (one change): its text, and the
/// thing laid (to choose it, and fill its window).
pub fn laid(text: &str, marker: Marker, point: [f32; 2], resident: &str) -> (String, Option<(usize, usize)>) {
    let line = line_at(text, marker, point, resident);
    let made = edit::with_line_added(text, &line, edit::is_life(&line));
    let found = edit::items(&made).into_iter().rev().find(|one| made.lines().nth(one.line - 1).is_some_and(|known| known.trim() == line.trim()));
    (made, found.map(|one| (one.line, one.at)))
}

/// An exit's line leading elsewhere: a game place by its number, a decor by
/// its name (and one of its entrances). None if that line is no exit, or nothing is said.
pub fn led_to(text: &str, number: usize, to: &str) -> Option<String> {
    let to = to.split_whitespace().collect::<Vec<_>>();
    if to.is_empty() || to.len() > 2 {
        return None;
    }
    let line = text.lines().nth(number.checked_sub(1)?)?;
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["exit", x, y, w, h, "to", ..] => Some(edit::with_lines(text, &[(number, format!("exit {x} {y} {w} {h} to {}", to.join(" ")))])),
        _ => None,
    }
}

/// Where an exit leads, as its line says (`224`, `my_harbour left`).
pub fn leads_to(line: &str) -> String {
    line.split_whitespace().skip_while(|word| *word != "to").skip(1).collect::<Vec<_>>().join(" ")
}

/// A resident's first line said made that (empty: none); the others kept.
pub fn with_first_said(text: &str, number: usize, said: &str) -> String {
    let mut lines = edit::speech(text, number);
    match (lines.is_empty(), said.trim().is_empty()) {
        (true, true) => return text.to_string(),
        (true, false) => lines.push(said.trim().to_string()),
        (false, _) => lines[0] = said.trim().to_string(),
    }
    edit::with_speech(text, number, &lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECOR: &str = "decor: tried 320 240\nspot spot1 40 40\nzone zone1 0 0 16 16\n\n[life]\nresident eevee 100 100 down\nsays Hello!\n";

    /// The decor as the decors' tool reads it: nothing wrong.
    fn reads(text: &str) {
        if let Err(why) = decor::read("tried", text) {
            panic!("line {}: {}\n{text}", why.line(), why.message);
        }
    }

    /// Each tool's line, its name new, at the point; a resident and an exit after `[life]`.
    #[test]
    fn each_marker_writes_its_line() {
        assert_eq!(line_at(DECOR, Marker::Resident, [120.4, 80.0], "piplup"), "resident piplup 120 80 down");
        assert_eq!(line_at(DECOR, Marker::Resident, [120.0, 80.0], " "), "resident pikachu 120 80 down");
        assert_eq!(line_at(DECOR, Marker::Spot, [10.0, 20.0], ""), "spot spot2 10 20");
        assert_eq!(line_at(DECOR, Marker::Entrance, [160.0, 236.0], ""), "entrance 160 236");
        assert_eq!(line_at(DECOR, Marker::Exit, [300.0, 120.0], ""), "exit 284 112 32 16 to 224");
        assert_eq!(line_at(DECOR, Marker::Zone, [100.0, 100.0], ""), "zone zone2 84 84 32 32");
        for marker in Marker::ALL {
            let (made, chosen) = laid(DECOR, marker, [150.0, 150.0], "piplup");
            reads(&made);
            let (line, _) = chosen.expect("laid");
            let written = made.lines().nth(line - 1).unwrap_or_default();
            assert!(written.starts_with(marker.word()), "{marker:?}: {written}");
            let life = made.lines().position(|line| line == "[life]").expect("[life]") + 1;
            assert_eq!(line > life, matches!(marker, Marker::Resident | Marker::Exit | Marker::Zone), "{marker:?}:\n{made}");
            assert_eq!(edit::items(&made).len(), edit::items(DECOR).len() + 1, "{marker:?}");
        }
    }

    /// A tool's word and key, each its own.
    #[test]
    fn each_marker_has_its_word_and_key() {
        for marker in Marker::ALL {
            assert_eq!(Marker::of_word(marker.word()), Some(marker));
            assert_eq!(Marker::ALL.iter().filter(|other| other.key().0 == marker.key().0).count(), 1);
        }
    }

    /// The essentials filled: a resident's Pokémon and first line, an entrance's name, an exit's way; the decor still reads.
    #[test]
    fn the_essentials_are_written() {
        let (made, chosen) = laid(DECOR, Marker::Resident, [200.0, 120.0], "pikachu");
        let (line, _) = chosen.expect("laid");
        let made = edit::renamed(&made, line, "piplup").expect("renamed");
        assert!(made.contains("resident piplup 200 120 down"), "{made}");
        let made = with_first_said(&made, line, "Welcome to the beach!");
        assert!(made.contains("resident piplup 200 120 down\nsays Welcome to the beach!"), "{made}");
        let made = with_first_said(&made, line, "Hi there.");
        assert!(made.contains("resident piplup 200 120 down\nsays Hi there.") && !made.contains("Welcome"), "{made}");
        reads(&made);
        // (Eevee's lines are his own: untouched.)
        assert!(made.contains("resident eevee 100 100 down\nsays Hello!"), "{made}");
        let (made, chosen) = laid(&made, Marker::Entrance, [160.0, 236.0], "");
        let made = edit::renamed(&made, chosen.expect("laid").0, "north gate").expect("named");
        assert!(made.contains("entrance 160 236 north_gate"), "{made}");
        assert_eq!(decor::doc::entrances(&made), vec![([160.0, 236.0], "north_gate".to_string())]);
        let (made, chosen) = laid(&made, Marker::Exit, [300.0, 120.0], "");
        let exit = chosen.expect("laid").0;
        let made = led_to(&made, exit, "my_harbour left").expect("led");
        assert!(made.contains("exit 284 112 32 16 to my_harbour left"), "{made}");
        assert_eq!(leads_to(made.lines().nth(exit - 1).unwrap_or_default()), "my_harbour left");
        assert_eq!(led_to(&made, exit, " "), None);
        reads(&made);
    }
}
