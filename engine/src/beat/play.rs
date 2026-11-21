//! What the player does in a scene.

use super::declare::{OnOff, declare};
use super::parse::Fields;
use super::*;
use crate::stage::Rect;

/// What the player does in a scene.
#[derive(Clone, Debug, PartialEq)]
pub enum PlayBeat {
    /// The player walks someone about with the arrows (none: no one any
    /// more); see `Stage::steer`.
    Control(Option<Who>),
    /// Those the player may walk about in turn, Tab passing from one to
    /// the next (none: only the one walked about).
    ControlTeam(Vec<Who>),
    /// Enter (and the arrows) counted with no one walked about (see
    /// `Director::press`, `Director::arrow`): a game of keys.
    Listen(bool),
    /// Where the one the player walks about may go: a box (none: anywhere).
    Bounds(Option<Rect>),
    /// A wall of the place: a box no one walked about goes into (none: no
    /// more walls).
    Wall(Option<Rect>),
    /// Collisions on or off, to the end of the scene: off, walks go
    /// straight where they are sent, through the place's walls and the
    /// scene's, no one steps aside, and places are taken as
    /// written (see `Stage::no_collisions`; a film's scenes open so, the
    /// game's with them).
    Collisions(bool),
    /// A wall by a name, put up or taken away (see `PlayBeat::Wall`).
    NamedWall { name: String, wall: Option<Rect> },
    /// A gauge kept on screen under the words of that place (see
    /// `HUD_PLACES`): a number against its most, in a colour (none:
    /// taken away).
    Gauge { at: usize, gauge: Option<GaugeOf> },
    /// Words kept on screen at the top, on the left, in the middle or on
    /// the right (`at`: 0, 1, 2; see `HUD_PLACES`), their numbers
    /// (`{name}`) always as they are now: a score, a count, time left
    /// (none: taken away).
    Hud { at: usize, text: Option<String> },
}

declare! {
    PlayBeat;
    hand: "bounds", "control",
        "gauge", "hud", "swapcontrol", "wall";
    "listen" => Listen(on: OnOff),
    "collisions" => Collisions(on: OnOff),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("bounds", &["left:word top:word right:word bottom:word", "off"], "they stay in this rectangle"),
    Form::new(
        "collisions",
        &["rest..."],
        "collisions, until the end of the scene. `off`: each goes straight where they are sent (`walkto`, `path`, entrances and exits), through the place's ground and the `wall`s; no one steps aside (`spacing` no longer plays); a place is taken as written, never pushed onto the free tile; a crowd takes all the room it needs; `check` no longer says anything about walls or bodies on top of each other. The ground still serves to choose where one comes in and goes out. `on`: all that comes back. A film's scenes open without (`off`), those of the game and of `.scene` files with",
    ),
    Form::new("control", &["who:code", "off"], "the player makes them walk with the arrows (Shift: running), while the scene goes on"),
    Form::new(
        "gauge",
        &["[rest...]", "[place:word] off"],
        "a bar under the words of this place, filled in proportion to the number (the max: a number or a name; green by default): hit points, flames that fade",
    ),
    Form::new(
        "hud",
        &["[rest...]", "[place:word] off"],
        "words kept at the top of the screen (on the right if you do not say where), their numbers always up to date: `hud left Lives: {lives}` and `hud Score: {score}` at the same time",
    ),
    Form::new(
        "listen",
        &["rest..."],
        "Enter and the arrows count without anyone being steered: a game of keys (`presses`, `press_clock`, `arrows`, `arrow`, `arrow_clock`)",
    ),
    Form::new(
        "swapcontrol",
        &["rest...", "off"],
        "the player can switch from one to the other with **Tab** (the first is steered, unless one of them already is); `controlled` says which",
    ),
    Form::new(
        "wall",
        &["left:word top:word right:word bottom:word", "off", "name:word left:word top:word right:word bottom:word", "name:word off"],
        "a wall of the place: they cannot go into it and slide along it; the gaze (`sees`) does not pass through it; those who flee (`avoid`) and those who look for their way (`walkto`) go around it (several at once; they hold for the place: a new backdrop clears them, like `bounds` and the `image`s)",
    ),
];

pub(super) fn read(fields: &Fields) -> Option<PlayBeat> {
    if let Some(beat) = PlayBeat::declared(fields) {
        return Some(beat);
    }
    // (Four numbers from there: a rectangle.)
    let four = |at: usize| Some([fields.real(at)?, fields.real(at + 1)?, fields.real(at + 2)?, fields.real(at + 3)?]);
    Some(match fields.verb()? {
        // (`gauge [place] <number> <most> [colour]`, `gauge [place] off`.)
        "gauge" => {
            let (at, from) = match fields.word(1).and_then(hud_place) {
                Some(at) => (at, 2),
                None => (2, 1),
            };
            let gauge = match fields.named(from, "a number's name")? {
                "off" | "-" => None,
                var => Some(GaugeOf {
                    var: var.to_string(),
                    most: fields.value(from + 1)?,
                    colour: fields.word(from + 2).and_then(colour_named).unwrap_or([80.0, 230.0, 100.0]).map(|part| part as u8),
                }),
            };
            PlayBeat::Gauge { at, gauge }
        }
        // (`hud <words>` on the right; `hud left <words>`...)
        "hud" => {
            let (at, words) = match fields.word(1).and_then(hud_place) {
                Some(at) => (at, fields.word(2)),
                None => (2, fields.word(1)),
            };
            PlayBeat::Hud { at, text: words.filter(|text| *text != "off").map(|text| text.trim_start().replace("\\n", "\n")) }
        }
        "swapcontrol" => PlayBeat::ControlTeam(match fields.word(1) {
            Some("off") => Vec::new(),
            _ => fields.group(1)?,
        }),
        // (`wall <name> <left> <top> <right> <bottom>`, `wall <name> off`: a
        // wall by a name, a door that opens.)
        "wall" if fields.word(1).is_some_and(|name| name != "off" && name.parse::<f32>().is_err()) => PlayBeat::NamedWall {
            name: fields.word(1)?.to_string(),
            wall: match fields.word(2) {
                Some("off") => None,
                _ => Some(four(2)?.into()),
            },
        },
        "wall" => PlayBeat::Wall(match fields.word(1) {
            Some("off" | "-") => None,
            _ => Some(four(1)?.into()),
        }),
        "control" => PlayBeat::Control(match fields.word(1) {
            Some("off") => None,
            _ => Some(fields.who(1)?),
        }),
        "bounds" => PlayBeat::Bounds(match fields.word(1) {
            Some("off") => None,
            _ => Some(four(1)?.into()),
        }),
        _ => return None,
    })
}
