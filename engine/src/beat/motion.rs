//! Who goes where, and how.

use super::declare::{Flag, IfAbsent, OnOff, declare};
use super::parse::Fields;
use super::*;

/// Who goes where, and how.
#[derive(Clone, Debug, PartialEq)]
pub enum MotionBeat {
    Go {
        who: Who,
        at: Point,
        frames: Frames,
        slide: bool,
    },
    /// Someone walks (or runs) to a place at a walker's pace.
    Walk {
        who: Who,
        at: Point,
        run: bool,
    },
    /// Someone walks (or runs) to a point going round the walls (see
    /// `PlayBeat::Wall`).
    WalkTo {
        who: Who,
        at: Point,
        run: bool,
    },
    /// Someone walks through several points in turn (or runs).
    Path {
        who: Who,
        points: Vec<Vec2>,
        run: bool,
    },
    /// Someone takes a few steps from where it stands (or runs).
    Step {
        who: Who,
        dx: f32,
        dy: f32,
        run: bool,
    },
    /// Someone's pace when it walks, in pixels a frame (a walker's: 1.2).
    Speed {
        who: Who,
        pace: f32,
    },
    /// Someone goes round a point that many times (the other way when
    /// less than 0), in that many frames.
    Orbit {
        who: Who,
        at: Point,
        turns: f32,
        frames: Frames,
    },
    /// Someone goes round someone else, wherever that one goes.
    OrbitAround {
        who: Who,
        other: Who,
        turns: f32,
        frames: Frames,
    },
    /// Someone stopped on its way goes on along it (see `BodyBeat::Stop`).
    Resume(Who),
    /// Someone follows someone else about (none: stops), stopping so far
    /// behind (none told: a step).
    Follow {
        who: Who,
        leader: Option<Who>,
        gap: Option<f32>,
    },
    /// Someone walks up to someone else, stopping beside them.
    Approach {
        who: Who,
        other: Who,
        frames: Frames,
    },
    /// Someone runs away from someone else, that far.
    Flee {
        who: Who,
        from: Who,
        distance: f32,
    },
    /// Someone runs up to someone else, stopping beside them, facing them.
    Chase {
        who: Who,
        other: Who,
    },
    /// They walk to stand in a row from a point, that far from one another,
    /// facing down.
    LineUp {
        who: Vec<Who>,
        at: Vec2,
        gap: Vec2,
    },
    /// They are put at once round a point, that far from it, facing it.
    Ring {
        who: Vec<Who>,
        at: Vec2,
        radius: f32,
    },
    /// A line: the first leads, each of the others walking where the one
    /// before walked, that far behind (none: no line any more).
    Train {
        gap: f32,
        who: Vec<Who>,
    },
    /// All but these leave the stage at once (see `crossto`).
    KeepOnly(Vec<Who>),
    /// Someone copying another as a reflection would (none: no more): it
    /// moves as much, the other way across, the same way up and down,
    /// turned likewise; walls stop it.
    Mirror {
        who: Who,
        model: Option<Who>,
    },
    /// Someone wanders about where it stands, never further than that
    /// (0: stops).
    Wander {
        who: Who,
        radius: f32,
    },
    /// Who walks steps round the others it meets on its way.
    Spacing(bool),
}

declare! {
    MotionBeat;
    hand: "follow", "keeponly", "lineup", "mirror",
        "orbit", "path", "ring", "run", "runpath", "runstep",
        "runto", "step", "train", "walk", "walkto";
    "go" => Go { who: Who, at: Point, frames: Frames, slide: Flag },
    "approach" => Approach { who: Who, other: Who, frames: Frames },
    "chase" => Chase { who: Who, other: Who },
    "flee" => Flee { who: Who, from: Who, distance: f32 = 48.0 },
    "wander" => Wander { who: Who, radius: f32 = 24.0 },
    "speed" => Speed { who: Who, pace: f32 },
    "resume" => Resume(who: Who),
    "spacing" => Spacing(on: IfAbsent<OnOff> = true),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("approach", &["who:code other:code frames:duration"], "goes and stands next to the other"),
    Form::new("chase", &["who:code other:code"], "runs to the other, stops next to them and looks at them"),
    Form::new("flee", &["who:code from:word [distance:number]"], "runs away from the other (48 pixels by default)"),
    Form::new(
        "follow",
        &["who:code leader:code [gap:number]", "who:code -"],
        "follows them everywhere (stops behind, so many pixels away, 22 otherwise, or at an angle if someone is already there) / stops",
    ),
    Form::new(
        "go",
        &["who:code x:coordinate y:coordinate frames:duration slide:word rest..."],
        "there in so many frames; with `1` they slide (without walking or turning)",
    ),
    Form::new("keeponly", &["rest..."], "all the others leave the scene at once"),
    Form::new("lineup", &["x:coordinate y:coordinate dx:number dy:number rest..."], "in a row from the point, spaced by (dx, dy), facing down"),
    Form::new(
        "mirror",
        &["who:code model:word", "who:code off"],
        "copies them like a reflection: walks as far, the other way from left to right, the same way from top to bottom, facing the same; walls and the `bounds` rectangle stop them",
    ),
    Form::new(
        "orbit",
        &["who:code other:code [laps:number] [frames:duration]", "who:code x:coordinate y:coordinate [laps:number] [frames:duration]"],
        "turns around the other, who may move",
    ),
    Form::new("path", &["who:code x1:coordinate y1:coordinate x2:coordinate y2:coordinate rest..."], "through several points in a row"),
    Form::new("resume", &["who:code"], "the one a `stop` halted on the way sets off again towards where they were going"),
    Form::new("ring", &["x:coordinate y:coordinate radius:number rest..."], "put at once in a circle around the point, facing it: a crowd already in place"),
    Form::new("run", &["who:code x:coordinate y:coordinate"], "walks / runs there, at their pace"),
    Form::new("runpath", &["rest..."], "through several points in a row"),
    Form::new("runstep", &["rest..."], "a few steps from where they are: `step pika -20 0`"),
    Form::new("runto", &["rest..."], "goes there going around the walls (they find their way)"),
    Form::new(
        "spacing",
        &["[rest...]"],
        "those who walk step aside from those they meet: two who face each other go around each other and still arrive on time; the walls of `wall` and those of the backdrop count. `orbit` and slides do not dodge: they push the others",
    ),
    Form::new("speed", &["who:code pixels:number per rest..."], "their pace (a walker: 1.2; running 2.2 times faster)"),
    Form::new("step", &["who:code dx:number dy:number"], "a few steps from where they are: `step pika -20 0`"),
    Form::new(
        "train",
        &["gap:number leader:code rest...", "off"],
        "a line: each walks where the leader went, at this gap from the one before (runs if too far behind); a parade, a train",
    ),
    Form::new("walk", &["who:code x:coordinate y:coordinate"], "walks / runs there, at their pace"),
    Form::new("walkto", &["who:code x:coordinate y:coordinate"], "goes there going around the walls (they find their way)"),
    Form::new("wander", &["who:code [radius:number]"], "strolls around where they are (24 pixels; `0`: stops); does not count for `waitall`"),
];

pub(super) fn read(fields: &Fields) -> Option<MotionBeat> {
    if let Some(beat) = MotionBeat::declared(fields) {
        return Some(beat);
    }
    let verb = fields.verb()?;
    Some(match verb {
        "keeponly" => MotionBeat::KeepOnly(fields.group(1)?),
        "mirror" => MotionBeat::Mirror { who: fields.who(1)?, model: if fields.is(2, "off") { None } else { Some(fields.who(2)?) } },
        "ring" => MotionBeat::Ring { at: Vec2::new(fields.real(1)?, fields.real(2)?), radius: fields.real(3)?, who: fields.group(4)? },
        "train" => match fields.word(1) {
            Some("off") => MotionBeat::Train { gap: 0.0, who: Vec::new() },
            _ => MotionBeat::Train { gap: fields.real(1)?.max(4.0), who: fields.group(2).filter(|who| who.len() >= 2)? },
        },
        "walkto" | "runto" => MotionBeat::WalkTo { who: fields.who(1)?, at: fields.point(2)?, run: verb == "runto" },
        "path" | "runpath" => {
            let numbers: Vec<f32> = (2..fields.len()).map(|at| fields.real(at)).collect::<Option<_>>()?;
            if numbers.is_empty() || !numbers.len().is_multiple_of(2) {
                return None;
            }
            let points = numbers.chunks(2).map(|pair| Vec2::new(pair[0], pair[1])).collect();
            MotionBeat::Path { who: fields.who(1)?, points, run: verb == "runpath" }
        }
        "lineup" => {
            MotionBeat::LineUp { at: Vec2::new(fields.real(1)?, fields.real(2)?), gap: Vec2::new(fields.real(3)?, fields.real(4)?), who: fields.group(5)? }
        }
        "step" | "runstep" => MotionBeat::Step { who: fields.who(1)?, dx: fields.real(2)?, dy: fields.real(3)?, run: verb == "runstep" },
        // (`orbit <who> <other> [turns] [frames]`: round someone.)
        "orbit" if fields.num(2).is_none() => MotionBeat::OrbitAround {
            who: fields.who(1)?,
            other: fields.who(2)?,
            turns: fields.real(3).unwrap_or(1.0),
            frames: Frames(fields.real(4).unwrap_or(120.0)),
        },
        "orbit" => MotionBeat::Orbit {
            who: fields.who(1)?,
            at: fields.point(2)?,
            turns: fields.real(4).unwrap_or(1.0),
            frames: Frames(fields.real(5).unwrap_or(120.0)),
        },
        "follow" => MotionBeat::Follow { who: fields.who(1)?, leader: fields.who(2), gap: fields.real(3) },
        "walk" | "run" => MotionBeat::Walk { who: fields.who(1)?, at: fields.point(2)?, run: verb == "run" },
        _ => return None,
    })
}
