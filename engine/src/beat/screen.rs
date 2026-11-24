//! What is shown: the place, the light, what is in the air, words over all.

use super::declare::{Bit, OnOff, Opt, Text, declare};
use super::parse::Fields;
use super::*;
use crate::stage::Halo;

/// What is shown: the place, the light, what is in the air, words over all.
#[derive(Clone, Debug, PartialEq)]
pub enum ScreenBeat {
    /// A level's picture; with it, who stood on the old one leaves, but
    /// for the one kept (the leader).
    Backdrop(u16, bool, Option<Who>),
    /// Another place's picture coming over the one shown, over so many
    /// frames: who stands there stays.
    Crossfade {
        level: u16,
        frames: Frames,
    },
    /// A dungeon's name on a dark screen, until the next backdrop.
    Banner(String),
    Fade {
        out: bool,
        white: bool,
        frames: Frames,
    },
    /// The screen goes to a part of its light (of 256).
    Dim {
        white: bool,
        frames: Frames,
        to: f32,
    },
    /// The light takes a colour (red, green, blue, and how much of it, of
    /// 255) in that many frames: a sunset, the night, a cave's glow.
    Tint {
        colour: [f32; 4],
        frames: Frames,
    },
    /// The screen flashes that colour, fading in that many frames.
    Flash {
        colour: [f32; 3],
        frames: Frames,
    },
    /// What is in the air over the scene.
    Weather(Weather),
    /// A soft light around someone or on a point, that wide, of that
    /// colour (several at once: lanterns at night); that wide 0: that one
    /// put out; none: all of them.
    Halo(Option<Halo>),
    /// Particles about someone or a point (see `PARTICLES`): how many,
    /// of what colour (none: their kind's own).
    Particles {
        kind: ParticleKind,
        on: Cam,
        count: u16,
        colour: Option<[u8; 3]>,
    },
    /// Particles coming off someone, wherever it goes (see `PARTICLES`): so
    /// many a second, of what colour, for so many frames (none: until
    /// told, see `Stage::emit`).
    Emitter {
        kind: ParticleKind,
        who: Who,
        each_second: f32,
        colour: Option<[u8; 3]>,
        frames: Option<Frames>,
    },
    /// Those that come off someone stop.
    EmitterOff(Who),
    /// The screen rippling that much (0 to 1), getting there over so many
    /// frames: a dream, a vision (0: still again).
    Wave {
        to: f32,
        frames: Frames,
    },
    /// A line of the fight's log, at the bottom of the screen, for so many
    /// frames.
    BattleLog {
        frames: Frames,
        text: String,
    },
    /// A picture shown instead of the scene, which goes on behind it (`over`:
    /// its light on the scene, what is dark of it leaving the scene seen).
    /// (`frames`: the frames it takes to come over the one shown.)
    Cutaway {
        level: u16,
        x: f32,
        y: f32,
        over: bool,
        frames: Frames,
    },
    /// The view on the picture shown glides there.
    CutawayPan {
        x: f32,
        y: f32,
        frames: Frames,
    },
    /// The scene seen again.
    CutawayOff,
    /// A dungeon's map in a corner, as the game's: the ground one may walk
    /// on, the team, the others, the stairs and what lies about; none: gone.
    /// Found as one goes (`discover`): only what the team has seen.
    Minimap {
        team: Option<Vec<Who>>,
        discover: bool,
    },
    /// The screen without its colours: a vision, a memory.
    Grey(bool),
    /// Everyone stops where they are, in the middle of a step: time stopped.
    Still(bool),
    /// A bolt of lightning striking someone or a point (see `bolt` in the
    /// files: with its flash and its thunder).
    Lightning(Cam),
    /// A picture from a file put on a point of the scene, by a name (none:
    /// that one taken away).
    Picture {
        name: String,
        file: Option<String>,
        at: Point,
    },
    /// The screen covered (or uncovered) by black coming from a side (0
    /// left, 1 right, 2 top, 3 bottom), over so many frames.
    Wipe {
        out: bool,
        side: u8,
        frames: Frames,
    },
    /// Words going up the screen from its bottom to past its top, over so
    /// many frames: the end's credits.
    Credits {
        frames: Frames,
        lines: Vec<String>,
    },
    /// A few words over the scene for that many frames (none: gone).
    /// (`typed`: written out a letter at a time.)
    Caption {
        frames: Frames,
        text: String,
        typed: bool,
    },
}

declare! {
    ScreenBeat;
    hand: "caption", "credits", "crossfade", "cutaway", "emit",
        "fade", "fadein", "fadeout", "flash", "halo", "image", "light",
        "lightning", "minimap", "particles", "tint", "typecaption", "wave",
        "weather", "whitein", "whiteout", "wipe";
    "backdrop" => Backdrop(level: u16, kept: Bit, leader: Opt<Who>),
    "banner" => Banner(name: Text = String::new()),
    "battlelog" => BattleLog { frames: Frames, text: Text },
    "dim" => Dim { white: Bit, frames: Frames, to: f32 },
    "grey" => Grey(on: OnOff),
    "still" => Still(on: OnOff),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("backdrop", &["place:word 1 -"], "shows the place's backdrop"),
    Form::new("banner", &["text..."], "a title on a black screen (the name of a dungeon)"),
    Form::new("battlelog", &["frames:duration text..."], "a line of the battle log, at the bottom of the screen (the last three stay)"),
    Form::new("caption", &["frames:duration text..."], "a caption at the top of the screen / that types itself letter by letter"),
    Form::new("credits", &["time:duration lines..."], "credits that rise from the bottom to the top of the screen during this time, lines separated by `\\n`"),
    Form::new("crossfade", &["place:word [time:duration]"], "the backdrop of another place comes over in a cross-fade, each staying there"),
    Form::new(
        "cutaway",
        &["[over] place:word x:coordinate y:coordinate [time:duration]", "off", "pan x:coordinate y:coordinate time:duration"],
        "a picture in place of the scene, which goes on behind; `over`: only its light passes over the scene; `time`: it fades in over the previous one",
    ),
    Form::new("dim", &["0 frames:duration light:word out of rest..."], "darkens (`light`, `tint`)"),
    Form::new("emit", &["kind:word who:code [per second] [colour:word] [time:duration] rest..."], "crisp particles come out of them, wherever they go"),
    Form::new("fade", &["out:word rest..."], "a fade (`fadein`, `fadeout`, `whitein`, `whiteout`)"),
    Form::new("fadein", &["frames:duration"], "fade from / to black"),
    Form::new("fadeout", &["frames:duration"], "fade from / to black"),
    Form::new("flash", &["colour:word [frames:duration]"], "a colour by its name (below); or `flash <r> <g> <b> <frames>`"),
    Form::new("grey", &["rest..."], "the screen loses its colours: a vision, a memory"),
    Form::new(
        "halo",
        &["who:code [radius:number] [colour:word]", "@ x:coordinate y:coordinate [radius:number] [colour:word]"],
        "a soft glow around someone (which it follows, even if they rise) or on a point (24 pixels, orange by default); several at once: lanterns at night; `halo <who> off` puts it out, `halo off` all of them",
    ),
    Form::new(
        "image",
        &["name:word file:word x:coordinate y:coordinate", "name:word off"],
        "a picture of your own (a PNG, path from the scene: `../images/sign.png`) put on a point: a sign, a drawing",
    ),
    Form::new("light", &["ambience:word [frames:duration]"], "`normal` (`day`), `dawn`, `evening`, `night`, `cave`, `storm`, `dream`, `fire`"),
    Form::new("lightning", &["who:code", "@ x:coordinate y:coordinate"], "the engine's command behind `bolt`"),
    Form::new(
        "minimap",
        &["who:code", "off"],
        "the map of the place at the top right, one pixel per walkable tile: those (the team) in yellow, the others in red, the stairs in white, the items on the ground in blue",
    ),
    Form::new(
        "particles",
        &["kind:word who:code [number:word] [colour:word]", "kind:word @ x:coordinate y:coordinate [number:word] [colour:word]"],
        "`firework` (bursts into colours and falls back), `sparks` (spurt out), `fireflies` (float and blink around), `embers` (rise), `smoke`, `bubbles`, `dust` (falls), `drops` (thrown out), `healing` (a glow that rises); in sharp pixels, `cinders` (about the body), `zap` (a spark, gone at once), `drip` (a drop off the head), `leaves` (falling from over the head); 40 by default",
    ),
    Form::new(
        "tint",
        &["r:number g:number b:number strength:number frames:duration", "colour:word strength:number frames:duration"],
        "tints the light (strength out of 255; 0: normal)",
    ),
    Form::new("typecaption", &["rest..."], "a caption at the top of the screen / that types itself letter by letter"),
    Form::new("wave", &["strength:number [time:duration]"], "the screen ripples (from 0 to 1; `wave 0`: sharp again): a dream, a vision"),
    Form::new("weather", &["name:word"], "`none`, `rain`, `snow`, `hail`, `sand`, `mist`, `sun`, `petals`, `sparkles`, `leaves`"),
    Form::new("whitein", &["frames:duration"], "fade from / to white"),
    Form::new("whiteout", &["frames:duration"], "fade from / to white"),
    Form::new(
        "wipe",
        &["rest..."],
        "the black comes in from one side and covers the screen (`out`), or withdraws (`in`) (`wipe close` / `open` in a film): to go from one place to another",
    ),
];

pub(super) fn read(fields: &Fields) -> Option<ScreenBeat> {
    if let Some(beat) = ScreenBeat::declared(fields) {
        return Some(beat);
    }
    // (Where it shows: `@ x y`, or someone; where what follows starts.)
    let on = |at: usize| match fields.word(at) {
        Some("@") => Some((Cam::At(fields.point(at + 1)?), at + 3)),
        _ => Some((Cam::Who(fields.who(at)?), at + 1)),
    };
    let kind = |at: usize| fields.lookup(at, "a kind of particles (dust, sparks, bubbles…)", particle_kind);
    let colour = |at: usize| fields.has(at).then(|| fields.lookup(at, "a colour", colour_named)).flatten();
    let verb = fields.verb()?;
    Some(match verb {
        "credits" => ScreenBeat::Credits {
            frames: Frames(fields.frames(1)? as f32),
            lines: fields.named(2, "words")?.trim_start().split("\\n").map(str::to_string).collect(),
        },
        "caption" | "typecaption" => {
            ScreenBeat::Caption { frames: Frames(fields.real(1)?), text: fields.word(2).unwrap_or_default().replace("\\n", "\n"), typed: verb == "typecaption" }
        }
        "fade" => ScreenBeat::Fade { out: fields.flag(1), white: fields.flag(2), frames: Frames(fields.real(3)?) },
        // (Plainer words for the fades of this project's scenes.)
        "fadein" => ScreenBeat::Fade { out: false, white: false, frames: Frames(fields.real(1)?) },
        "fadeout" => ScreenBeat::Fade { out: true, white: false, frames: Frames(fields.real(1)?) },
        "whiteout" => ScreenBeat::Fade { out: true, white: true, frames: Frames(fields.real(1)?) },
        "whitein" => ScreenBeat::Fade { out: false, white: true, frames: Frames(fields.real(1)?) },
        // (`particles <kind> <who|@ x y> [count] [colour]`.)
        "particles" => {
            let kind = kind(1)?;
            let (on, rest) = on(2)?;
            ScreenBeat::Particles { kind, on, count: fields.number(rest).unwrap_or(40), colour: colour(rest + 1).map(|colour| colour.map(|part| part as u8)) }
        }
        // (`emit <kind> <who> [a second] [colour] [time]`, `emit off <who>`.)
        "emit" if fields.is(1, "off") => ScreenBeat::EmitterOff(fields.who(2)?),
        "emit" => ScreenBeat::Emitter {
            kind: kind(1)?,
            who: fields.who(2)?,
            each_second: fields.real(3).unwrap_or(8.0).max(0.1),
            colour: colour(4).map(|colour| colour.map(|part| part as u8)),
            frames: fields.has(5).then(|| fields.frames(5)).flatten().map(|frames| Frames(frames as f32)),
        },
        "cutaway" if fields.is(1, "off") => ScreenBeat::CutawayOff,
        // (`minimap [discover] <team...>`, `minimap off`.)
        "minimap" => {
            let discover = fields.is(1, "discover");
            let from = 1 + usize::from(discover);
            ScreenBeat::Minimap {
                team: match fields.word(1) {
                    Some("off" | "-") => None,
                    _ => Some((from..fields.len()).map(|at| fields.who(at)).collect::<Option<Vec<Who>>>()?),
                },
                discover,
            }
        }
        "cutaway" if fields.is(1, "pan") => ScreenBeat::CutawayPan { x: fields.real(2)?, y: fields.real(3)?, frames: Frames(fields.frames(4)? as f32) },
        "cutaway" => {
            let over = fields.is(1, "over");
            let from = 1 + usize::from(over);
            let frames = fields.has(from + 3).then(|| fields.frames(from + 3)).flatten().unwrap_or(0) as f32;
            ScreenBeat::Cutaway { level: fields.number(from)?, x: fields.real(from + 1)?, y: fields.real(from + 2)?, over, frames: Frames(frames) }
        }
        "wave" => {
            ScreenBeat::Wave { to: fields.real(1)?.clamp(0.0, 1.0), frames: Frames(fields.has(2).then(|| fields.frames(2)).flatten().unwrap_or(30) as f32) }
        }
        "crossfade" => {
            ScreenBeat::Crossfade { level: fields.number(1)?, frames: Frames(fields.has(2).then(|| fields.frames(2)).flatten().unwrap_or(60) as f32) }
        }
        "lightning" => ScreenBeat::Lightning(on(1)?.0),
        // (`image <name> <file> <x> <y>`, `image <name> off`.)
        "image" => ScreenBeat::Picture {
            name: fields.named(1, "a name")?.to_string(),
            file: Some(fields.named(2, "a file")?.to_string()).filter(|file| file != "off"),
            at: match fields.word(2) {
                Some("off") => Point::at(Vec2::ZERO),
                _ => fields.point(3)?,
            },
        },
        "wipe" => ScreenBeat::Wipe {
            out: fields.lookup(1, "in or out", |way| match way {
                "out" => Some(true),
                "in" => Some(false),
                _ => None,
            })?,
            side: match fields.has(2) {
                false => 0,
                true => fields.lookup(2, "a side (left, right, top, bottom)", |side| match side {
                    "left" => Some(0),
                    "right" => Some(1),
                    "top" => Some(2),
                    "bottom" => Some(3),
                    _ => None,
                })?,
            },
            frames: Frames(fields.has(3).then(|| fields.frames(3)).flatten().unwrap_or(30) as f32),
        },
        // (`halo <who|@ x y> [width] [colour]`, `halo <who> off`, `halo off`.)
        "halo" => {
            let (on, rest) = match fields.word(1) {
                Some("off" | "-") => return Some(ScreenBeat::Halo(None)),
                _ => on(1)?,
            };
            let colour = colour(rest + 1).unwrap_or([255.0, 200.0, 120.0]).map(|part| part as u8);
            let radius = match fields.word(rest) {
                Some("off") => 0.0,
                _ => fields.real(rest).unwrap_or(24.0),
            };
            ScreenBeat::Halo(Some(Halo { on, radius, colour }))
        }
        // (`tint <r> <g> <b> <strength> <frames>`, or `tint <colour> <strength> <frames>`.)
        "tint" => match fields.word(1).and_then(colour_named) {
            Some([red, green, blue]) => ScreenBeat::Tint { colour: [red, green, blue, fields.real(2)?], frames: Frames(fields.real(3)?) },
            None => ScreenBeat::Tint { colour: [fields.real(1)?, fields.real(2)?, fields.real(3)?, fields.real(4)?], frames: Frames(fields.real(5)?) },
        },
        "flash" => match fields.word(1).and_then(colour_named) {
            Some(colour) => ScreenBeat::Flash { colour, frames: Frames(fields.real(2).unwrap_or(20.0)) },
            None => ScreenBeat::Flash { colour: [fields.real(1)?, fields.real(2)?, fields.real(3)?], frames: Frames(fields.real(4)?) },
        },
        "weather" => ScreenBeat::Weather(fields.lookup(1, "a weather (rain, snow…)", weather_named)?),
        "light" => {
            ScreenBeat::Tint { colour: fields.lookup(1, "a light (dawn, dusk, night…)", light_named)?, frames: Frames(fields.real(2).unwrap_or(60.0)) }
        }
        _ => return None,
    })
}
