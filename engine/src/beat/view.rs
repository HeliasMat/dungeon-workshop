//! Where the view looks, and how fast time goes.

use super::declare::{Bit, IfAbsent, OnOff, Opt, declare};
use super::parse::Fields;
use super::*;
use crate::stage::{Ease, Lit};

/// Where the view looks, and how fast time goes.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewBeat {
    Camera(Option<Cam>),
    /// How the glides from now on go: slowly at their ends, evenly, or
    /// quickly then slowing.
    Ease(Ease),
    /// The view glides to a point or to someone, in that many frames.
    Pan {
        to: Cam,
        frames: Frames,
    },
    /// The same, and it stays there, not following whom it went to.
    PanStays {
        to: Cam,
        frames: Frames,
    },
    /// The view leaves room for the dialogue box when a line shows (or
    /// not): `Frame` and `KeepFrame` always do.
    BoxRoom(bool),
    /// The view comes closer (2: twice as big) or goes back (1), in that
    /// many frames.
    Zoom {
        to: f32,
        frames: Frames,
    },
    /// The view glides and comes as close as it can while showing all of
    /// them, in that many frames.
    Frame {
        who: Vec<Who>,
        /// Points of the place shown with them (`@x,y`): a lamp, a door.
        at: Vec<Vec2>,
        frames: Frames,
        /// Never closer than that (`max:0.5`).
        most: Option<f32>,
    },
    /// The view keeps them all in sight, however they move, as close as it
    /// can (none: as it was).
    KeepFrame(Option<Vec<Who>>),
    /// How big someone shows, wide and high, for the view to fit it.
    Body {
        who: Who,
        size: Vec2,
    },
    /// The view following someone catches up with them in about that many
    /// frames (0: keeps on them).
    CamLag(f32),
    /// The view glides to a point or someone, stays there that many
    /// frames, then glides back.
    Glance {
        to: Cam,
        frames: Frames,
        hold: f32,
    },
    /// The view shakes along an axis (1 sideways, 2 up and down).
    Shake {
        axis: u16,
        strength: f32,
        frames: Option<Frames>,
    },
    /// The view quakes that hard, less and less, for that many frames.
    Quake {
        strength: f32,
        frames: Frames,
    },
    /// The view closes on a point (or someone) to a black screen, or opens
    /// from there, in that many frames, as a circle.
    Iris {
        open: bool,
        frames: Frames,
        on: Option<Who>,
    },
    /// Black bands above and below, as in a film (or not).
    Letterbox(bool),
    /// Time goes that much faster on the scene (0.5: slowed down; 1: as
    /// usual), sprites and sounds alike: the game sets its clock so.
    Tempo(f32),
    /// The scene goes dark but for a circle of light on a point or on
    /// someone, that wide (none: the light comes back).
    Spotlight(Option<Lit>),
    /// Another hole in the dark round the spotlight's (see `ViewBeat::Spotlight`).
    SpotlightAdd(Cam, f32),
    /// The view glides to who speaks only if it does not show them.
    KeepSpeaker(bool),
}

declare! {
    ViewBeat;
    hand: "camera", "ease", "frame", "glance", "iris", "keepframe", "pan",
        "spotlight", "tempo";
    "camlag" => CamLag(frames: f32),
    "zoom" => Zoom { to: f32, frames: Frames },
    "body" => Body { who: Who, size: Vec2 },
    "shake" => Shake { axis: u16, strength: f32, frames: Opt<Frames> },
    "quake" => Quake { strength: f32, frames: Frames = Frames(40.0) },
    "letterbox" => Letterbox(on: Bit),
    "boxroom" => BoxRoom(on: IfAbsent<OnOff> = true),
    "keepspeaker" => KeepSpeaker(on: IfAbsent<OnOff> = true),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new(
        "body",
        &["who:code wide:word high:word"],
        "the size of a body on screen, in pixels: `frame` and `keepframe` make it fit (`film/` writes it on its own for the big ones)",
    ),
    Form::new(
        "boxroom",
        &["[rest...]"],
        "the view moves away from the dialogue box when a line shows, and stays away between two lines; `frame` and `keepframe` do it on their own",
    ),
    Form::new("camera", &["who:code", "@ x:coordinate y:coordinate"], "the view on someone (and follows them) / on a point, at once"),
    Form::new("camlag", &["frames:duration"], "the view that follows someone catches up with them smoothly (`0`: stuck to them)"),
    Form::new(
        "ease",
        &["rest..."],
        "how the `pan`s that start afterwards slide: slowly at the ends (smooth, like the game), at an even pace, or fast then slowing down",
    ),
    Form::new(
        "frame",
        &["frames:duration rest... [rest2...]"],
        "the view slides and shows them all, bodies included (see `body`), with a margin; it moves back if it must, never closer than 1 (never zooming: the view only slides); `max:0.5`: never closer than this zoom",
    ),
    Form::new(
        "glance",
        &["who:code [frames:duration] [wait]", "@ x:coordinate y:coordinate rest..."],
        "the view goes to take a look, stays there (60 frames), then comes back",
    ),
    Form::new("iris", &["out [frames:duration] [who:code]", "in rest..."], "the circle that closes onto black (onto someone), then opens again"),
    Form::new(
        "keepframe",
        &["rest...", "off"],
        "keeps them all in view continuously, wherever they go; the view moves back if their bodies do not fit, never closer than 1",
    ),
    Form::new("keepspeaker", &["[rest...]"], "the view slides to the speaker, only if it does not show them"),
    Form::new("letterbox", &["1", "0"], "black cinema bars"),
    Form::new(
        "pan",
        &[
            "who:code frames:duration",
            "@ x:coordinate y:coordinate frames:duration",
            "who:code frames:duration fixed",
            "@ x:coordinate y:coordinate frames:duration fixed",
        ],
        "the view slides there smoothly",
    ),
    Form::new("quake", &["strength:number [frames:duration]", ""], "an earthquake that dies down (40 frames)"),
    Form::new("shake", &["axis:word strength:number [frames:duration]"], "a shake (strength 0: stop; with a duration, stops on its own)"),
    Form::new(
        "spotlight",
        &[
            "who:code [radius:number]",
            "@ x:coordinate y:coordinate [radius:number]",
            "off",
            "plus who:code [radius:number]",
            "plus @ x:coordinate y:coordinate [radius:number]",
        ],
        "everything darkens except a circle of light, which follows the person",
    ),
    Form::new("tempo", &["speed:number"], "the scene's time slower or faster (`0.4` slowed; `1` normal)"),
    Form::new(
        "zoom",
        &["closeness:word frames:duration"],
        "brings the view closer (2: twice as big; 1: normal). The original game shows its scenes without zoom, and all those in this folder are at 1",
    ),
];

pub(super) fn read(fields: &Fields) -> Option<ViewBeat> {
    if let Some(beat) = ViewBeat::declared(fields) {
        return Some(beat);
    }
    // (Where the view goes: `@ x y`, or someone; where what follows starts.)
    let on = |at: usize| match fields.word(at) {
        Some("@") => Some((Cam::At(fields.point(at + 1)?), at + 3)),
        _ => Some((Cam::Who(fields.who(at)?), at + 1)),
    };
    Some(match fields.verb()? {
        "camera" => ViewBeat::Camera(match fields.word(1) {
            Some("@") => Some(Cam::At(fields.point(2)?)),
            _ if fields.has(1) => fields.who(1).map(Cam::Who),
            _ => return None,
        }),
        "keepframe" => ViewBeat::KeepFrame(match fields.word(1) {
            Some("off") => None,
            _ => Some(fields.group(1)?),
        }),
        "frame" => {
            // (`frame <frames> <who...> [@x,y...] [max:0.5]`.)
            let most = fields.word(fields.len().checked_sub(1)?).and_then(|last| last.strip_prefix("max:")).and_then(|most| most.parse::<f32>().ok());
            let end = fields.len() - usize::from(most.is_some());
            let point = |word: &str| {
                let (x, y) = word.strip_prefix('@')?.split_once(',')?;
                Some(Vec2::new(x.parse().ok()?, y.parse().ok()?))
            };
            let at: Vec<Vec2> = (2..end).filter_map(|at| fields.word(at).and_then(point)).collect();
            let who: Vec<Who> =
                (2..end).filter(|at| !fields.word(*at).is_some_and(|word| word.starts_with('@'))).map(|at| fields.who(at)).collect::<Option<_>>()?;
            (!who.is_empty() || !at.is_empty()).then_some(ViewBeat::Frame { frames: Frames(fields.real(1)?), who, at, most })?
        }
        // (`spotlight plus <who|@ x y> [radius]`: another hole in the dark.)
        "spotlight" if matches!(fields.word(1), Some("plus" | "add")) => {
            let (on, rest) = on(2)?;
            ViewBeat::SpotlightAdd(on, fields.real(rest).unwrap_or(30.0))
        }
        "spotlight" => ViewBeat::Spotlight(match fields.word(1) {
            Some("off" | "-") => None,
            _ => {
                let (on, rest) = on(1)?;
                Some(Lit { on, radius: fields.real(rest).unwrap_or(40.0) })
            }
        }),
        "glance" => {
            let (to, rest) = on(1)?;
            ViewBeat::Glance { to, frames: Frames(fields.real(rest).unwrap_or(30.0)), hold: fields.real(rest + 1).unwrap_or(60.0) }
        }
        "tempo" => ViewBeat::Tempo(fields.real(1)?.clamp(0.1, 4.0)),
        "ease" if fields.len() >= 2 => match Ease::read(&fields.words_from(1)?)? {
            (ease, used) if used + 1 == fields.len() => ViewBeat::Ease(ease),
            _ => return None,
        },
        "pan" if fields.is(fields.len() - 1, "fixed") => {
            let (to, rest) = on(1)?;
            ViewBeat::PanStays { to, frames: Frames(fields.real(rest)?) }
        }
        "pan" => {
            let (to, rest) = on(1)?;
            ViewBeat::Pan { to, frames: Frames(fields.real(rest)?) }
        }
        "iris" => ViewBeat::Iris {
            open: fields.lookup(1, "in or out", |way| match way {
                "in" => Some(true),
                "out" => Some(false),
                _ => None,
            })?,
            frames: Frames(fields.real(2).unwrap_or(40.0)),
            on: fields.has(3).then(|| fields.who(3)).flatten(),
        },
        _ => return None,
    })
}
