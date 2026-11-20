//! The beats of what the scene keeps on screen and in the way: words
//! kept, a gauge, a wall by a name.

use engine::{Beat, GaugeOf, PlayBeat, Rect};

/// Words kept on screen at that place (`engine::hud_place`; none:
/// taken away).
pub fn hud(at: usize, text: Option<String>) -> Beat {
    Beat::Play(PlayBeat::Hud { at, text })
}

/// A gauge kept on screen at that place, of a number and its most (none:
/// taken away).
pub fn gauge(at: usize, gauge: Option<GaugeOf>) -> Beat {
    Beat::Play(PlayBeat::Gauge { at, gauge })
}

/// Collisions on or off, to the end of the scene: off, walks go straight
/// and places are as written.
pub fn collisions(on: bool) -> Beat {
    Beat::Play(PlayBeat::Collisions(on))
}

/// A wall by a name, in the way there (none: it opens).
pub fn named_wall(name: impl Into<String>, wall: Option<Rect>) -> Beat {
    Beat::Play(PlayBeat::NamedWall { name: name.into(), wall })
}
