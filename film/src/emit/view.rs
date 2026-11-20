//! The beats of the view: where it looks.

use engine::{Beat, Cam, Ease, Frames, ViewBeat, Who};

/// The view goes to someone or a point in so many frames, stays so long,
/// and comes back.
pub fn glance(to: Cam, frames: f32, hold: f32) -> Beat {
    Beat::View(ViewBeat::Glance { to, frames: Frames(frames), hold })
}

/// The view goes to someone or a point at once (none: back to its own).
pub fn camera(to: Option<Cam>) -> Beat {
    Beat::View(ViewBeat::Camera(to))
}

/// The ground shakes that hard for so many frames.
pub fn quake(strength: f32, frames: f32) -> Beat {
    Beat::View(ViewBeat::Quake { strength, frames: Frames(frames) })
}

/// The scene in the dark but a hole about someone or a point, that wide.
pub fn spotlight(on: Cam, radius: f32) -> Beat {
    Beat::View(ViewBeat::Spotlight(Some(engine::Lit { on, radius })))
}

/// The dark over the scene lifts: no more spotlight.
pub fn spotlight_off() -> Beat {
    Beat::View(ViewBeat::Spotlight(None))
}

/// The view opens from black (or closes to it) as a circle, in so many
/// frames, on someone if told.
pub fn iris(open: bool, frames: f32, on: Option<Who>) -> Beat {
    Beat::View(ViewBeat::Iris { open, frames: Frames(frames), on })
}

/// The view glides to who speaks only if it does not show them (or always).
pub fn keep_speaker(on: bool) -> Beat {
    Beat::View(ViewBeat::KeepSpeaker(on))
}

/// The view shakes along an axis that hard, for so many frames if told.
pub fn shake(axis: u16, strength: f32, frames: Option<f32>) -> Beat {
    Beat::View(ViewBeat::Shake { axis, strength, frames: frames.map(Frames) })
}

/// The view keeps them all in sight as they move (none: as it was).
pub fn keep_frame(who: Option<Vec<Who>>) -> Beat {
    Beat::View(ViewBeat::KeepFrame(who))
}

/// The view comes as close as it can showing them all, in so many frames,
/// never closer than `most` if told.
pub fn frame(who: Vec<Who>, frames: f32, most: Option<f32>) -> Beat {
    frame_with(who, Vec::new(), frames, most)
}

/// The same, points of the place shown with them (`frame riolu lamp`).
pub fn frame_with(who: Vec<Who>, at: Vec<glam::Vec2>, frames: f32, most: Option<f32>) -> Beat {
    Beat::View(ViewBeat::Frame { who, at, frames: Frames(frames), most })
}

/// The view leaves room for the dialogue box when a line shows (or not).
pub fn box_room(on: bool) -> Beat {
    Beat::View(ViewBeat::BoxRoom(on))
}

/// The view glides to someone or a point in so many frames, and stays there
/// if told (not following whom it went to).
pub fn pan(to: Cam, frames: f32, stays: bool) -> Beat {
    Beat::View(if stays { ViewBeat::PanStays { to, frames: Frames(frames) } } else { ViewBeat::Pan { to, frames: Frames(frames) } })
}

/// Another hole in the dark, about someone or a point, that wide.
pub fn spotlight_add(on: Cam, radius: f32) -> Beat {
    Beat::View(ViewBeat::SpotlightAdd(on, radius))
}

/// How the view's glides go from now on: evenly, quickly then slowing, or
/// smoothly (as untold).
pub fn ease(ease: Ease) -> Beat {
    Beat::View(ViewBeat::Ease(ease))
}
