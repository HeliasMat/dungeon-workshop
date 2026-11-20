//! The beats over the whole screen: particles, flashes.

use engine::{Beat, Cam, Frames, Halo, ParticleKind, Point, ScreenBeat, Weather, Who};

use crate::film::Film;
use crate::screen::BREAK;

/// So many particles of a kind about someone or a point, of that colour
/// (none: their kind's own).
pub fn particles(kind: ParticleKind, on: Cam, count: u16, colour: Option<[u8; 3]>) -> Beat {
    Beat::Screen(ScreenBeat::Particles { kind, on, count, colour })
}

/// Particles coming off someone wherever it goes, so many a second, of that
/// colour, for so many frames (none: until told to stop).
pub fn emitter(kind: ParticleKind, who: Who, each_second: f32, colour: Option<[u8; 3]>, frames: Option<f32>) -> Beat {
    Beat::Screen(ScreenBeat::Emitter { kind, who, each_second, colour, frames: frames.map(Frames) })
}

/// The particles coming off someone stop.
pub fn emitter_off(who: Who) -> Beat {
    Beat::Screen(ScreenBeat::EmitterOff(who))
}

/// The screen flashes that colour (`engine::colour_named`) for so many frames.
pub fn flash(colour: [f32; 3], frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Flash { colour, frames: Frames(frames) })
}

/// A bolt of lightning strikes someone or a point.
pub fn lightning(on: Cam) -> Beat {
    Beat::Screen(ScreenBeat::Lightning(on))
}

/// The screen fades out (or in), to black or white, in so many frames.
pub fn fade(out: bool, white: bool, frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Fade { out, white, frames: Frames(frames) })
}

/// A picture of the game instead of the scene (`over`: its light on the
/// scene), coming over the one shown in so many frames.
pub fn cutaway(level: u16, x: f32, y: f32, over: bool, frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Cutaway { level, x, y, over, frames: Frames(frames) })
}

/// The view on the picture shown glides there in so many frames.
pub fn cutaway_pan(x: f32, y: f32, frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::CutawayPan { x, y, frames: Frames(frames) })
}

/// The scene seen again.
pub fn cutaway_off() -> Beat {
    Beat::Screen(ScreenBeat::CutawayOff)
}

/// The screen without its colours, or with them again.
pub fn grey(on: bool) -> Beat {
    Beat::Screen(ScreenBeat::Grey(on))
}

/// The screen ripples that much (0 to 1), getting there in so many frames.
pub fn wave(to: f32, frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Wave { to, frames: Frames(frames) })
}

/// A few words over the scene for that many frames, written out a letter at
/// a time if `typed`.
pub fn caption(frames: f32, text: impl Into<String>, typed: bool) -> Beat {
    Beat::Screen(ScreenBeat::Caption { frames: Frames(frames), text: text.into(), typed })
}

/// The end's credits going up the screen over so many frames.
pub fn credits(frames: f32, lines: Vec<String>) -> Beat {
    Beat::Screen(ScreenBeat::Credits { frames: Frames(frames), lines })
}

/// The light takes a colour (`engine::light_named`) in so many frames.
pub fn tint(colour: [f32; 4], frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Tint { colour, frames: Frames(frames) })
}

/// A light about someone or a point, that wide, of that colour (none: its
/// own, a warm white).
pub fn halo(on: Cam, radius: f32, colour: Option<[u8; 3]>) -> Beat {
    Beat::Screen(ScreenBeat::Halo(Some(Halo { on, radius, colour: colour.unwrap_or([255, 200, 120]) })))
}

/// The lights about anyone go out.
pub fn halo_off() -> Beat {
    Beat::Screen(ScreenBeat::Halo(None))
}

/// The whole stage stands still, or moves again.
pub fn still(on: bool) -> Beat {
    Beat::Screen(ScreenBeat::Still(on))
}

/// The sky does that (`engine::weather_named`).
pub fn weather(weather: Weather) -> Beat {
    Beat::Screen(ScreenBeat::Weather(weather))
}

impl Film {
    /// Words over the scene for that many frames (`typed`: a letter at a
    /// time), their lines cut at `BREAK`; `{name:pikachu}` in words not
    /// typed by its code, as the game puts names in.
    pub fn caption(&mut self, frames: f32, text: &str, typed: bool) {
        let text = if typed { text.to_string() } else { self.names_in(text) };
        self.beat(caption(frames, text.replace(BREAK, "\n"), typed));
    }
}

/// A picture from a file put on a point of the scene, by a name (no file:
/// that one taken away).
pub fn picture(name: impl Into<String>, file: Option<String>, at: glam::Vec2) -> Beat {
    picture_point(name, file, at.into())
}

/// A picture from a file put on a point of the scene (found as it plays, maybe).
pub fn picture_point(name: impl Into<String>, file: Option<String>, at: Point) -> Beat {
    Beat::Screen(ScreenBeat::Picture { name: name.into(), file, at })
}

/// The scene's backdrop: a place of the game (`shown`: as the game shows it),
/// seen from someone if told.
pub fn backdrop(level: u16, shown: bool, on: Option<Who>) -> Beat {
    Beat::Screen(ScreenBeat::Backdrop(level, shown, on))
}

/// Black wipes over the screen from a side (0 left, 1 right, 2 top, 3
/// bottom), or away to it, in so many frames.
pub fn wipe(out: bool, side: u8, frames: f32) -> Beat {
    Beat::Screen(ScreenBeat::Wipe { out, side, frames: Frames(frames) })
}

/// A line in the log of a fight, for so many frames.
pub fn battle_log(frames: f32, text: impl Into<String>) -> Beat {
    Beat::Screen(ScreenBeat::BattleLog { frames: Frames(frames), text: text.into() })
}

/// The floor's map in a corner, the team on it (none: taken away);
/// `discover`: only what they have seen.
pub fn minimap(team: Option<Vec<Who>>, discover: bool) -> Beat {
    Beat::Screen(ScreenBeat::Minimap { team, discover })
}
