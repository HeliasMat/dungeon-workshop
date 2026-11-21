//! The numbers and words a scene keeps.

use super::declare::{Text, declare};
use super::parse::Fields;
use super::*;

/// The numbers and words a scene keeps.
#[derive(Clone, Debug, PartialEq)]
pub enum DataBeat {
    /// A number given a value, added to, or drawn by chance (below that).
    Set {
        var: String,
        value: Value,
    },
    Add {
        var: String,
        value: Value,
    },
    Random {
        var: String,
        from: i32,
        to: i32,
    },
    /// A number given what two others make: added (`+`), taken away (`-`),
    /// multiplied (`*`), divided (`/`), what is left of a division (`%`),
    /// the smaller or the larger (`min`, `max`).
    Calc {
        var: String,
        a: Value,
        op: String,
        b: Value,
    },
    /// A number given how far apart two of the stage are, in pixels.
    Measure {
        var: String,
        a: Who,
        b: Who,
    },
    Position {
        who: Who,
        x: String,
        y: String,
    },
    /// A number given whether one sees another: near enough, in front of
    /// it within that angle (degrees, the whole cone), no wall between.
    Sees {
        var: String,
        who: Who,
        other: Who,
        distance: f32,
        angle: f32,
    },
    /// Words kept by a name (`{name}` in words puts them in).
    SetText {
        var: String,
        text: String,
    },
    /// A countdown, in frames (none: stopped): the seconds left given as
    /// `timer`, 0 once over.
    /// A countdown: the number it is read by (`timer`, or `timer_<name>`
    /// for one named), its frames (none: stopped).
    Timer {
        var: String,
        frames: Option<Frames>,
    },
}

declare! {
    DataBeat;
    hand: "calc", "random", "settext", "timer";
    "set" => Set { var: Text, value: Value },
    "add" => Add { var: Text, value: Value = Value::parse("1") },
    "measure" => Measure { var: Text, a: Who, b: Who },
    "sees" => Sees { var: Text, who: Who, other: Who, distance: f32 = 64.0, angle: f32 = 90.0 },
    "position" => Position { who: Who, x: Text, y: Text },
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("add", &["name:word [value:number]"], "adds (1 by default; `-1` takes away)"),
    Form::new(
        "calc",
        &["name:word a:word op:word b:number"],
        "`+` `-` `*` `/` (decimal: 7 / 2 is 3.5) `div` (whole: 7 div 2 is 3) `%` (the remainder) `min` `max` (a division by 0 gives 0)",
    ),
    Form::new("measure", &["name:word a:word b:number"], "the distance between the two, in pixels"),
    Form::new("position", &["who:code x:coordinate y:coordinate"], "two numbers take where they stand"),
    Form::new("random", &["name:word n:number", "name:word a:word b:number"], "at random from 0 to n-1 / from a to b"),
    Form::new(
        "sees",
        &["name:word watcher:word who:code [distance:number] [angle:number]"],
        "1 if the watcher sees them: close enough (64 pixels), in front of them in this cone (90 degrees), with no wall between them",
    ),
    Form::new("set", &["name:word value:number"], "a value (a number, or another name)"),
    Form::new("settext", &["name:word words..."], "words kept under this name, put into texts by `{name}`; kept in the save"),
    Form::new(
        "timer",
        &["time:duration", "off"],
        "a countdown: `timer` gives the seconds left, 0 at the end (`hud {timer} s`, `waituntil timer == 0`); as many as you like, named: `timer bomb 30s` gives `timer_bomb` (`timer(bomb)` in a film), `timer bomb off`",
    ),
];

pub(super) fn read(fields: &Fields) -> Option<DataBeat> {
    if let Some(beat) = DataBeat::declared(fields) {
        return Some(beat);
    }
    Some(match fields.verb()? {
        "settext" => DataBeat::SetText { var: fields.named(1, "a name")?.to_string(), text: fields.named(2, "words")?.trim().to_string() },
        // (`timer 30s`, `timer off`; named, as many as wanted: `timer bomb 30s`.)
        "timer" => {
            let (var, time) = match (fields.has(1), fields.has(2)) {
                (true, true) => (format!("{}_{}", crate::director::TIMER, fields.named(1, "a name")?), 2),
                (true, false) => (crate::director::TIMER.to_string(), 1),
                _ => return None,
            };
            DataBeat::Timer {
                var,
                frames: match fields.word(time) {
                    Some("off") => None,
                    _ => Some(Frames(fields.frames(time)? as f32)),
                },
            }
        }
        "calc" => DataBeat::Calc {
            var: fields.named(1, "a name")?.to_string(),
            a: fields.value(2)?,
            op: fields
                .lookup(3, "a sign (+ - * / div % min max)", |op| matches!(op, "+" | "-" | "*" | "/" | "div" | "%" | "min" | "max").then(|| op.to_string()))?,
            b: fields.value(4)?,
        },
        // (`random <name> <n>`: 0 to n-1; `random <name> <a> <b>`: a to b.)
        "random" => {
            let var = || Some(fields.named(1, "a name")?.to_string());
            match (fields.whole(2)?, fields.has(3).then(|| fields.whole(3)).flatten()) {
                (from, Some(to)) => DataBeat::Random { var: var()?, from: from.min(to), to: from.max(to) },
                (below, None) => DataBeat::Random { var: var()?, from: 0, to: (below - 1).max(0) },
            }
        }
        _ => return None,
    })
}
