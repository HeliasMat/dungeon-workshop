//! What the export tells of a film, as notes: each on its line, of a kind,
//! how much is lost (changed to fit, left out, not possible on DS), a short
//! sentence and what the DS does instead. The Workshop marks the Script's
//! lines with them and lists them in its DS Report; `export-ds` prints them.

use std::collections::BTreeSet;
use std::fmt;

/// How much of a line the DS loses, the least first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fate {
    /// Nothing lost: something put in the ROM for it (a Pokémon SpriteCollab adds).
    Added,
    /// Played, with less or otherwise than the film shows.
    Changed,
    /// Not played on DS (`--partial`: the rest of the film goes on).
    LeftOut,
    /// The DS has nothing for it: a word that does not export, or what
    /// the export cannot know (left out too, with `--partial`).
    Impossible,
}

impl Fate {
    /// How the Report and the margin say it.
    pub fn word(self) -> &'static str {
        match self {
            Fate::Added => "added",
            Fate::Changed => "changed",
            Fate::LeftOut => "left out",
            Fate::Impossible => "not possible",
        }
    }
}

/// What a note is about: the Report groups them by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A word of the language that exports with a loss, or not at all.
    Word,
    /// A line said in a bubble.
    Bubble,
    /// An item held, given, dropped, thrown or put.
    Item,
    /// A pose a species' sprite lacks.
    Pose,
    /// A Pokémon the ROM has no sprite for, drawn as another.
    Species,
    /// A Pokémon SpriteCollab adds, put in the ROM.
    Added,
    /// More Pokémon at once than Sky shows.
    Crowd,
    /// The camera: a zoom, a frame wider than the screen.
    Camera,
    /// The top screen, the screen's effects.
    Screen,
    Weather,
    /// A state's effect, words floating up.
    Effect,
    /// A face, a line's box.
    Speech,
    Sound,
    Fight,
    /// A decor of one's own made a map of Sky's.
    Decor,
    /// Lines played beside the script, in Sky's routines.
    Timing,
    /// An engine's step with no operation of Sky's.
    Engine,
    /// Where someone is, a variable, a test: what the export can't know.
    Logic,
}

impl Kind {
    /// Its heading in the Report.
    pub fn title(self) -> &'static str {
        match self {
            Kind::Word => "Words",
            Kind::Bubble => "Bubbles",
            Kind::Item => "Items",
            Kind::Pose => "Poses",
            Kind::Species => "Pokémon drawn as another",
            Kind::Added => "Pokémon added to the ROM",
            Kind::Crowd => "Crowds",
            Kind::Camera => "Camera",
            Kind::Screen => "Screen",
            Kind::Weather => "Weather",
            Kind::Effect => "Effects",
            Kind::Speech => "Speech",
            Kind::Sound => "Sound",
            Kind::Fight => "Fights",
            Kind::Decor => "Decors",
            Kind::Timing => "Timing",
            Kind::Engine => "Engine commands",
            Kind::Logic => "Positions and variables",
        }
    }
}

/// One thing the export tells: on which line of the film (none: the film
/// as a whole), of what kind, how much is lost, what (a short sentence),
/// and what the DS does instead (empty: nothing).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    pub line: Option<usize>,
    pub kind: Kind,
    pub fate: Fate,
    pub message: String,
    pub instead: String,
}

impl Note {
    pub fn new(line: i32, kind: Kind, fate: Fate, message: impl Into<String>, instead: impl Into<String>) -> Note {
        Note { line: usize::try_from(line).ok().filter(|line| *line > 0), kind, fate, message: message.into(), instead: instead.into() }
    }
}

impl fmt::Display for Note {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(out, "line {line}: ")?;
        }
        write!(out, "{} ({})", self.message, self.fate.word())?;
        if !self.instead.is_empty() {
            write!(out, "; on DS: {}", self.instead)?;
        }
        Ok(())
    }
}

/// What the export tells of a film: its notes (sorted by line, each once),
/// how many of its lines play, the Pokémon put in the ROM.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub notes: Vec<Note>,
    /// The film's lines that play (a scene, a line said, a word...).
    pub lines: usize,
    /// The Pokémon SpriteCollab adds the film casts, put in the ROM.
    pub added: usize,
}

/// The Report in numbers: of the lines that play, those as written, those
/// with a change at most, those left out (or not possible), the Pokémon added.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub lines: usize,
    pub as_written: usize,
    pub changed: usize,
    pub left_out: usize,
    pub impossible: usize,
    pub added: usize,
}

impl Summary {
    /// The lines as written, out of a hundred (all of them when none play).
    pub fn percent(&self) -> usize {
        if self.lines == 0 { 100 } else { self.as_written * 100 / self.lines }
    }

    /// One line: "on DS: 92% as written, 12 changed, 3 left out, 4 new Pokémon added".
    pub fn said(&self) -> String {
        let mut said = format!("on DS: {}% as written", self.percent());
        if self.changed > 0 {
            said.push_str(&format!(", {} changed", self.changed));
        }
        if self.left_out > 0 {
            said.push_str(&format!(", {} left out", self.left_out));
        }
        if self.impossible > 0 {
            said.push_str(&format!(", {} not possible", self.impossible));
        }
        if self.added > 0 {
            said.push_str(&format!(", {} new Pokémon added", self.added));
        }
        said
    }
}

/// What an engine's steps left out are said as (`writer::unexported`): a
/// line's word, and its step.
const STEP: (&str, &str) = (": its ", " isn't played");

/// The engine's steps left out of one line said in one note: "hold: its
/// carry, takeable and thing name aren't played".
fn merged_steps(notes: Vec<Note>) -> Vec<Note> {
    let mut out: Vec<Note> = Vec::new();
    for note in notes {
        let step = |note: &Note| -> Option<(String, String)> {
            let (word, rest) = note.message.split_once(STEP.0)?;
            Some((word.to_string(), rest.strip_suffix(STEP.1)?.to_string()))
        };
        let last = out.iter_mut().rev().find(|one| one.line == note.line && one.kind == note.kind && one.fate == note.fate && note.kind == Kind::Engine);
        if let (Some(last), Some((word, what))) = (last, step(&note)) {
            let (first, steps) = last.message.split_once(STEP.0).map(|(first, rest)| (first.to_string(), rest.to_string())).unwrap_or_default();
            if first == word {
                let mut parts: Vec<String> = steps
                    .trim_end_matches(STEP.1)
                    .trim_end_matches(" aren't played")
                    .split(", ")
                    .flat_map(|part| part.split(" and "))
                    .map(str::to_string)
                    .collect();
                parts.push(what);
                let (rest, end) = parts.split_at(parts.len() - 1);
                last.message = format!("{word}{}{} and {} aren't played", STEP.0, rest.join(", "), end[0]);
                continue;
            }
        }
        out.push(note);
    }
    out
}

impl Report {
    /// The notes made a report: sorted by line, the same one told once; a
    /// word's own loss left out where a note of its own says it better
    /// (a bubble, an item, the camera, the weather).
    pub fn of(mut notes: Vec<Note>, lines: usize, added: usize) -> Report {
        notes.sort();
        notes.dedup();
        let specific: BTreeSet<usize> =
            notes.iter().filter(|note| matches!(note.kind, Kind::Bubble | Kind::Item | Kind::Camera | Kind::Weather)).filter_map(|note| note.line).collect();
        notes.retain(|note| !(note.kind == Kind::Word && note.fate == Fate::Changed && note.line.is_some_and(|line| specific.contains(&line))));
        // (A word the DS has nothing for: its steps aren't told one by one.)
        let refused: BTreeSet<usize> =
            notes.iter().filter(|note| matches!(note.kind, Kind::Word | Kind::Engine) && note.fate == Fate::Impossible).filter_map(|note| note.line).collect();
        notes.retain(|note| {
            note.fate == Fate::Impossible
                || !(note.line.is_some_and(|line| refused.contains(&line))
                    && matches!(note.kind, Kind::Engine | Kind::Item | Kind::Camera | Kind::Logic | Kind::Screen | Kind::Sound | Kind::Speech | Kind::Effect))
        });
        // (An item's steps: the item's own note says it.)
        let items: BTreeSet<usize> = notes.iter().filter(|note| note.kind == Kind::Item).filter_map(|note| note.line).collect();
        notes.retain(|note| !(note.kind == Kind::Engine && note.fate == Fate::LeftOut && note.line.is_some_and(|line| items.contains(&line))));
        Report { notes: merged_steps(notes), lines, added }
    }

    /// The notes of that line.
    pub fn on_line(&self, line: usize) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |note| note.line == Some(line))
    }

    /// The worst a line loses (none: played as written; added is not a loss).
    pub fn worst(&self, line: usize) -> Option<Fate> {
        self.on_line(line).map(|note| note.fate).filter(|fate| *fate != Fate::Added).max()
    }

    /// The notes of each kind, the kinds in their order, with how many.
    pub fn by_kind(&self) -> Vec<(Kind, Vec<&Note>)> {
        let mut kinds: Vec<(Kind, Vec<&Note>)> = Vec::new();
        let mut sorted: Vec<&Note> = self.notes.iter().collect();
        sorted.sort_by_key(|note| (note.kind, std::cmp::Reverse(note.fate), note.line));
        for note in sorted {
            match kinds.last_mut() {
                Some((kind, notes)) if *kind == note.kind => notes.push(note),
                _ => kinds.push((note.kind, vec![note])),
            }
        }
        kinds
    }

    /// In numbers: each line counted once, by the worst it loses.
    pub fn summary(&self) -> Summary {
        let lined: BTreeSet<usize> = self.notes.iter().filter_map(|note| note.line).collect();
        let (mut changed, mut left_out, mut impossible) = (0, 0, 0);
        for line in lined {
            match self.worst(line) {
                Some(Fate::Changed) => changed += 1,
                Some(Fate::LeftOut) => left_out += 1,
                Some(Fate::Impossible) => impossible += 1,
                _ => {}
            }
        }
        let lines = self.lines.max(changed + left_out + impossible);
        Summary { lines, as_written: lines - changed - left_out - impossible, changed, left_out, impossible, added: self.added }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(line: i32, kind: Kind, fate: Fate, message: &str) -> Note {
        Note::new(line, kind, fate, message, "")
    }

    /// The same note told twice is one; a word's own loss is left out where
    /// a note of its own (a bubble) says it; each line counted once, by the
    /// worst it loses.
    #[test]
    fn a_report_tells_each_note_once_and_counts_each_line_by_its_worst() {
        let notes = vec![
            note(3, Kind::Bubble, Fate::Changed, "a line in a bubble"),
            note(3, Kind::Word, Fate::Changed, "bubble: no bubble"),
            note(3, Kind::Bubble, Fate::Changed, "a line in a bubble"),
            note(5, Kind::Word, Fate::Changed, "laugh: no arc"),
            note(5, Kind::Pose, Fate::Changed, "Pikachu's Sky sprite has no Twirl pose"),
            note(7, Kind::Item, Fate::LeftOut, "the item isn't shown"),
            note(7, Kind::Word, Fate::Changed, "hold: the item not shown"),
            note(9, Kind::Word, Fate::Impossible, "“fish” isn't on DS yet"),
            note(2, Kind::Added, Fate::Added, "grookey (species 2810): a Pokémon SpriteCollab adds"),
            note(0, Kind::Crowd, Fate::LeftOut, "25 Pokémon from the start"),
        ];
        let report = Report::of(notes, 20, 1);
        assert_eq!(report.on_line(3).count(), 1, "{:?}", report.notes);
        assert_eq!(report.on_line(5).count(), 2, "a pose and the word's own loss: {:?}", report.notes);
        assert_eq!(report.on_line(7).map(|note| note.kind).collect::<Vec<_>>(), [Kind::Item]);
        assert_eq!(report.worst(7), Some(Fate::LeftOut));
        assert_eq!(report.worst(2), None, "added is no loss");
        assert_eq!(report.worst(4), None);
        let summary = report.summary();
        assert_eq!(summary, Summary { lines: 20, as_written: 16, changed: 2, left_out: 1, impossible: 1, added: 1 });
        assert_eq!(summary.percent(), 80);
        assert_eq!(summary.said(), "on DS: 80% as written, 2 changed, 1 left out, 1 not possible, 1 new Pokémon added");
        let kinds: Vec<(Kind, usize)> = report.by_kind().into_iter().map(|(kind, notes)| (kind, notes.len())).collect();
        assert_eq!(kinds, [(Kind::Word, 2), (Kind::Bubble, 1), (Kind::Item, 1), (Kind::Pose, 1), (Kind::Added, 1), (Kind::Crowd, 1)]);
        assert_eq!(report.notes.iter().find(|note| note.line.is_none()).map(Note::to_string).as_deref(), Some("25 Pokémon from the start (left out)"));
        assert_eq!(
            Note::new(4, Kind::Bubble, Fate::Changed, "a line in a bubble", "a box line").to_string(),
            "line 4: a line in a bubble (changed); on DS: a box line"
        );
    }

    /// The engine's steps left out of one line: one note.
    #[test]
    fn the_steps_of_one_line_are_one_note() {
        let step = |what: &str| Note::new(4, Kind::Engine, Fate::LeftOut, format!("lift: its {what} isn't played"), "nothing");
        let report = Report::of(vec![step("set"), step("path"), step("shadow")], 5, 0);
        assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
        assert_eq!(report.notes[0].message, "lift: its path, set and shadow aren't played");
    }

    #[test]
    fn a_film_that_loses_nothing_is_all_as_written() {
        let summary = Report::of(Vec::new(), 12, 0).summary();
        assert_eq!(summary.percent(), 100);
        assert_eq!(summary.said(), "on DS: 100% as written");
    }
}
