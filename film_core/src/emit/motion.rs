//! The beats of who goes where.

use engine::{Beat, Frames, MotionBeat, Point, Who};

/// Someone goes up to someone else, in so many frames.
pub fn approach(who: Who, other: Who, frames: f32) -> Beat {
    Beat::Motion(MotionBeat::Approach { who, other, frames: Frames(frames) })
}

/// Someone walks (or runs) to a point.
pub fn walk(who: Who, x: f32, y: f32, run: bool) -> Beat {
    walk_point(who, glam::Vec2::new(x, y).into(), run)
}

/// Someone walks (or runs) to a point (found as it plays, maybe).
pub fn walk_point(who: Who, at: Point, run: bool) -> Beat {
    Beat::Motion(MotionBeat::Walk { who, at, run })
}

/// Someone glides to a point in so many frames (`slide`: still looking where
/// it looked).
pub fn go(who: Who, x: f32, y: f32, frames: f32, slide: bool) -> Beat {
    go_point(who, glam::Vec2::new(x, y).into(), frames, slide)
}

/// Someone glides to a point (found as it plays, maybe) in so many frames.
pub fn go_point(who: Who, at: Point, frames: f32, slide: bool) -> Beat {
    Beat::Motion(MotionBeat::Go { who, at, frames: Frames(frames), slide })
}

/// Someone takes a few steps from where it stands (or runs).
pub fn step(who: Who, dx: f32, dy: f32, run: bool) -> Beat {
    Beat::Motion(MotionBeat::Step { who, dx, dy, run })
}

/// Someone runs up to someone else, stopping beside them, facing them.
pub fn chase(who: Who, other: Who) -> Beat {
    Beat::Motion(MotionBeat::Chase { who, other })
}

/// Someone wanders about where it stands, never further than that (0: stops).
pub fn wander(who: Who, radius: f32) -> Beat {
    Beat::Motion(MotionBeat::Wander { who, radius })
}

/// Someone goes round a point that many times, in that many frames.
pub fn orbit(who: Who, x: f32, y: f32, turns: f32, frames: f32) -> Beat {
    Beat::Motion(MotionBeat::Orbit { who, at: glam::Vec2::new(x, y).into(), turns, frames: Frames(frames) })
}

/// Someone goes round a point (found as it plays, maybe) that many times,
/// in that many frames.
pub fn orbit_point(who: Who, at: Point, turns: f32, frames: f32) -> Beat {
    Beat::Motion(MotionBeat::Orbit { who, at, turns, frames: Frames(frames) })
}

/// Someone goes round someone else that many times, wherever it goes.
pub fn orbit_around(who: Who, other: Who, turns: f32, frames: f32) -> Beat {
    Beat::Motion(MotionBeat::OrbitAround { who, other, turns, frames: Frames(frames) })
}

/// They walk in file, each that far behind the one before (none: no file).
pub fn train(gap: f32, who: Vec<Who>) -> Beat {
    Beat::Motion(MotionBeat::Train { gap, who })
}

/// Those on stage keep apart from each other (or no more).
pub fn spacing(on: bool) -> Beat {
    Beat::Motion(MotionBeat::Spacing(on))
}

/// Someone walks at that pace from now on.
pub fn speed(who: Who, pace: f32) -> Beat {
    Beat::Motion(MotionBeat::Speed { who, pace })
}

/// Someone follows someone else about, so far behind if told (none: stops).
pub fn follow(who: Who, leader: Option<Who>, gap: Option<f32>) -> Beat {
    Beat::Motion(MotionBeat::Follow { who, leader, gap })
}
