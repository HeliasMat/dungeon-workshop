//! The engine's codes of what is wrong in a scene (a `diagnostics::Diagnostic`
//! says it), and a line that cannot be read.

use std::fmt;

/// The stable codes of the problems, the one place they are given: `E-`
/// for an error (the film does not play as written), `R-` for a remark (it
/// plays as asked, to be looked at), then the family (`PROP`, `VALUE`,
/// `NODE`...), then what. A code is never renamed nor given another
/// meaning: new ones are added. [`codes::ALL`] tells each.
pub mod codes {
    /// No property has that name (the nearest are given).
    pub const PROP_UNKNOWN: &str = "E-PROP-UNKNOWN";
    /// That kind of node has no such property (`radius` on a body).
    pub const PROP_KIND: &str = "E-PROP-KIND";
    /// A property given without its value.
    pub const VALUE_MISSING: &str = "E-VALUE-MISSING";
    /// A value that is not of the property's type (`red` for an angle).
    pub const VALUE_TYPE: &str = "E-VALUE-TYPE";
    /// A value no node can take: not a finite number, a negative size.
    pub const VALUE_IMPOSSIBLE: &str = "E-VALUE-IMPOSSIBLE";
    /// No node of the scene has that name.
    pub const NODE_UNKNOWN: &str = "E-NODE-UNKNOWN";
    /// No kind of node made by `node` has that name.
    pub const NODE_KIND: &str = "E-NODE-KIND";
    /// A node's name already someone's or something's of the scene.
    pub const NODE_TAKEN: &str = "E-NODE-TAKEN";
    /// A node that cannot hang on another (the caption, an emitter, a crowd).
    pub const ATTACH_KIND: &str = "E-ATTACH-KIND";
    /// A node hung on itself, or on one that hangs on it.
    pub const ATTACH_LOOP: &str = "E-ATTACH-LOOP";
    /// A rig's file not found, or not read.
    pub const RIG_FILE: &str = "E-RIG-FILE";
    /// A line of a rig's file that is wrong: its line said.
    pub const RIG_LINE: &str = "E-RIG-LINE";
    /// A character's sprite sheet not found, or its cells wrong.
    pub const SHEET_FILE: &str = "E-CHARACTER-SHEET";
    /// An animation of a character wrong: its cells, its pace, its row.
    pub const SHEET_ANIM: &str = "E-CHARACTER-ANIM";
    /// A track of a property that does not animate (`prop` sets it).
    pub const TRACK_STILL: &str = "E-TRACK-PROP-STILL";
    /// An ease no one knows.
    pub const EASE_UNKNOWN: &str = "E-EASE-UNKNOWN";
    /// A track waited for that never ends (it loops).
    pub const TRACK_ENDLESS: &str = "R-TRACK-ENDLESS";
    /// A line of `anim` or of a `keys` block that does not read.
    pub const TRACK_FORM: &str = "E-TRACK-FORM";
    /// A key of a `keys` block that does not read: its time, its value.
    pub const TRACK_KEY: &str = "E-TRACK-KEY";
    /// A path no `path` line made.
    pub const PATH_UNKNOWN: &str = "E-PATH-UNKNOWN";
    /// A path's points that do not read, or fewer than two.
    pub const PATH_POINTS: &str = "E-PATH-POINTS";
    /// A value past what the registry advises: played as asked.
    pub const OUT_OF_RANGE: &str = "R-OUT-OF-RANGE";

    /// What a rehearsal sees as the film plays, by its kind (`Seen::kind`):
    /// the kind, its code as a remark, what it is.
    pub const SEEN: &[(&str, &str, &str)] = &[
        ("view", "R-PLAY-VIEW", "who speaks is out of the picture, or hidden by the box"),
        ("absent", "R-PLAY-ABSENT", "the one spoken to is not in the picture"),
        ("wall", "R-PLAY-WALL", "a body at rest inside the backdrop"),
        ("on", "R-PLAY-ON", "two bodies at rest on one another"),
        ("stuck", "R-PLAY-STUCK", "a walk that does not move forward"),
        ("through", "R-PLAY-THROUGH", "a walk through the backdrop"),
        ("fast", "R-PLAY-FAST", "a line gone before it could be read"),
        ("pops", "R-PLAY-POPS", "someone appears or disappears all at once, in the picture"),
        ("sound", "R-PLAY-SOUND", "a sound waited for whose length is not known"),
        ("frame", "R-PLAY-FRAME", "the shot or the fight keeps someone who stays out of the picture"),
        ("field", "R-PLAY-FIELD", "someone of the crowd on a fight's field"),
        ("thing", "R-PLAY-THING", "a thing put inside the backdrop"),
        ("cut", "R-PLAY-CUT", "a large body cut by the edge of the picture"),
        ("crowd", "R-PLAY-CROWD", "the crowd, placed otherwise than written"),
        ("line", "R-PLAY-LINE", "what the game notes about a line"),
        ("item", "R-PLAY-ITEM", "an item held or put otherwise than written"),
        ("painted", "R-PLAY-PAINTED", "a variant shown from a side it is not painted from"),
        ("place", "R-PLAY-PLACE", "a place found otherwise than written"),
    ];
    /// A place the stage could not find as written, as the film played.
    pub const SEEN_WRONG: &str = "E-PLAY-PLACE";
    /// What a rehearsal saw, of no kind above.
    pub const SEEN_OTHER: &str = "R-PLAY-OTHER";

    /// The code of what a rehearsal saw: by its kind, a remark; wrong (the
    /// film does not play as written), [`SEEN_WRONG`].
    pub fn seen(kind: &str, wrong: bool) -> &'static str {
        match wrong {
            true => SEEN_WRONG,
            false => SEEN.iter().find(|(known, ..)| *known == kind).map_or(SEEN_OTHER, |(_, code, _)| code),
        }
    }

    /// The codes of what a rehearsal sees, with what each means.
    pub fn seen_all() -> Vec<(&'static str, &'static str)> {
        let seen = SEEN.iter().map(|(_, code, meaning)| (*code, *meaning));
        seen.chain([(SEEN_WRONG, "a place the scene does not find as written"), (SEEN_OTHER, "what the game sees, of another kind")]).collect()
    }

    /// Every code of the engine, with what it means: those given here, then
    /// those of what a rehearsal sees.
    pub fn every() -> Vec<(&'static str, &'static str)> {
        ALL.iter().copied().chain(seen_all()).collect()
    }

    /// Every code, with what it means.
    pub const ALL: &[(&str, &str)] = &[
        (PROP_UNKNOWN, "no property has this name (the nearest are given)"),
        (PROP_KIND, "this kind of node does not have this property"),
        (VALUE_MISSING, "a property without its value"),
        (VALUE_TYPE, "a value that is not of the property's type"),
        (VALUE_IMPOSSIBLE, "a value no node can take: not a finite number, a negative size"),
        (NODE_UNKNOWN, "no node of the scene has this name"),
        (NODE_KIND, "no kind of node has this name"),
        (NODE_TAKEN, "this name is already someone's or something's in the scene"),
        (ATTACH_KIND, "this node does not attach (the caption, an emitter, a crowd)"),
        (ATTACH_LOOP, "a node attached to itself, or to a node already hanging on it"),
        (RIG_FILE, "a rig's file not found or not readable"),
        (RIG_LINE, "a wrong line in a rig's file (its line is given)"),
        (SHEET_FILE, "a character's sheet not found, or its cells wrong"),
        (SHEET_ANIM, "a character's animation wrong: its cells, its pace, its row"),
        (TRACK_STILL, "a track of a property that does not animate (set sets it at once)"),
        (EASE_UNKNOWN, "an unknown ease (the “Eases” list in film/FILMS.md)"),
        (TRACK_ENDLESS, "a looping track never ends: the film does not wait for it (“&”, “stop anim”)"),
        (TRACK_FORM, "a badly formed anim or keys line: <prop> [from] -> <to> in 2s"),
        (TRACK_KEY, "an unreadable key of a keys block: <t> <value> [ease]"),
        (PATH_UNKNOWN, "a path no “path” line made"),
        (PATH_POINTS, "the points of a path: x y, x y, … (two at least)"),
        (OUT_OF_RANGE, "a value outside what the registry advises, played as it is"),
    ];
}

/// Why a line cannot be read as a beat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// There is nothing on it.
    Empty,
    /// No command has that name; the nearest one, if any is near.
    Unknown { word: String, close: Option<&'static str> },
    /// The command is known, what follows it is not what it wants.
    Malformed { word: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Empty => write!(out, "empty line"),
            ParseError::Unknown { word, close: Some(close) } => write!(out, "unknown command “{word}” (“{close}”?)"),
            ParseError::Unknown { word, close: None } => write!(out, "unknown command “{word}”"),
            ParseError::Malformed { word } => write!(out, "“{word}” badly written (see COMMANDS.md)"),
        }
    }
}

impl std::error::Error for ParseError {}

/// What fills a hole of a message the engine writes: words, or someone it
/// names.
pub(crate) enum Arg {
    Words(String),
    Who(crate::who::Who),
}

impl From<crate::who::Who> for Arg {
    fn from(who: crate::who::Who) -> Self {
        Arg::Who(who)
    }
}

impl From<String> for Arg {
    fn from(words: String) -> Self {
        Arg::Words(words)
    }
}

impl From<&str> for Arg {
    fn from(words: &str) -> Self {
        Arg::Words(words.to_string())
    }
}

/// A message the engine writes, its holes (`{}`) filled in turn: those it
/// names are written as a film's tools read them (`species m25`), and a
/// Pokémon of the scene is kept typed (`Said::mentions`), to be said by the
/// name the film gives (`film::Written::by_name`). See `told!`.
pub(crate) fn telling(template: &str, args: impl IntoIterator<Item = Arg>) -> diagnostics::Said {
    use crate::who::Who;
    let mut args = args.into_iter();
    let mut pieces = template.split("{}");
    let mut said = diagnostics::Said::default().words(pieces.next().unwrap_or_default());
    for piece in pieces {
        said = match args.next() {
            Some(Arg::Words(words)) => said.words(&words),
            Some(Arg::Who(who)) => {
                let written = format!("species {who}");
                match who {
                    Who::Monster(species) => said.naming(&written, species, 0),
                    Who::Another { species, copy } => said.naming(&written, species, copy),
                    _ => said.words(&written),
                }
            }
            None => said,
        };
        said = said.words(piece);
    }
    said
}

/// `told!("{} walks through the backdrop, {}", who, where)`: a message, as
/// [`telling`] writes it.
macro_rules! told {
    ($template:literal $(, $arg:expr)* $(,)?) => {
        $crate::problem::telling($template, [$($crate::problem::Arg::from($arg)),*])
    };
}
pub(crate) use told;

/// A point as whoever writes a scene reads it: `(300, 150)`.
pub(crate) fn point(at: glam::Vec2) -> String {
    format!("({}, {})", at.x.round(), at.y.round())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_said() {
        for (at, (code, meaning)) in codes::ALL.iter().enumerate() {
            assert!(code.starts_with("E-") || code.starts_with("R-"), "{code}");
            assert!(!meaning.is_empty());
            assert!(codes::ALL[..at].iter().all(|(other, _)| other != code), "{code} twice");
        }
        let every = codes::every();
        for (at, (code, _)) in every.iter().enumerate() {
            assert!(every[..at].iter().all(|(other, _)| other != code), "{code} twice");
        }
    }

    #[test]
    fn what_is_seen_has_a_code_by_its_kind() {
        assert_eq!(codes::seen("view", false), "R-PLAY-VIEW");
        assert_eq!(codes::seen("place", true), codes::SEEN_WRONG);
        assert_eq!(codes::seen("whatever", false), codes::SEEN_OTHER);
    }

    #[test]
    fn those_a_message_names_are_kept_typed_as_it_writes_them() {
        use crate::who::Who;
        let said = told!("{} hides {} from {}.", Who::Monster(25), Who::Another { species: 25, copy: 2 }, Who::Character(0));
        assert_eq!(said.text, "species m25 hides species m25.2 from species 0.");
        let named: Vec<(u16, u16, &str)> = said.mentions.iter().map(|mention| (mention.species, mention.copy, mention.at.of(&said.text))).collect();
        assert_eq!(named, [(25, 0, "species m25"), (25, 2, "species m25.2")]);
    }
}
