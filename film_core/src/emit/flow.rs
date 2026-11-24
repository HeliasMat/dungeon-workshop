//! The beats of the scene's course: waits, what happens later, a line
//! whose numbers are found while it plays.

use engine::{Beat, FlowBeat, Frames, Point, Who};

use crate::film::Film;

/// That many 60ths of a second pass.
pub fn wait(frames: u32) -> Beat {
    Beat::Flow(FlowBeat::Wait(Frames(frames as f32)))
}

/// The scene waits until someone has got where it was going.
pub fn wait_for(who: Who) -> Beat {
    Beat::Flow(FlowBeat::WaitFor(who))
}

/// The scene waits until the screen's fade is done.
pub fn wait_fade() -> Beat {
    Beat::Flow(FlowBeat::WaitFade)
}

/// A beat that happens that many frames later (and up to `more` frames
/// after that, drawn), whatever the scene does meanwhile.
pub fn after(frames: f32, more: f32, beat: Beat) -> Beat {
    Beat::Flow(FlowBeat::After { frames: Frames(frames), more: Frames(more), beat: Box::new(beat) })
}

impl Film {
    /// A wait of so many frames. Less than nothing is no wait: told.
    pub fn wait(&mut self, frames: i64) {
        let Ok(frames) = u32::try_from(frames) else {
            self.not_a(frames, "a duration");
            return;
        };
        self.beat(wait(frames));
    }

    /// A wait said by `wait 2s`, at the film's pace (`pace`, which
    /// changes only those): never under a frame.
    pub fn wait_paced(&mut self, frames: i64) {
        match u32::try_from(frames) {
            Ok(frames) if self.pace != 1.0 => self.wait(i64::from(((frames as f32 * self.pace).round() as u32).max(1))),
            _ => self.wait(frames),
        }
    }
}

/// The scene waits until everyone has got where it was going.
pub fn wait_all() -> Beat {
    Beat::Flow(FlowBeat::WaitAll)
}

/// The scene waits until a number is that (`emit::number`, `emit::var`).
pub fn wait_until(var: impl Into<String>, value: engine::Value) -> Beat {
    let test = engine::Test { any: vec![vec![engine::Condition { var: var.into(), compare: engine::Compare::Equal, value }]] };
    Beat::Flow(FlowBeat::WaitUntil(test))
}

/// The scene waits until a number has come to that (`clock >= 720`).
pub fn wait_reached(var: impl Into<String>, at: f64) -> Beat {
    let condition = engine::Condition { var: var.into(), compare: engine::Compare::GreaterOrEqual, value: engine::Value::Number(at) };
    Beat::Flow(FlowBeat::WaitUntil(engine::Test { any: vec![vec![condition]] }))
}

/// The scene waits as long as a sound lasts, played so many half tones higher.
pub fn wait_sound(of: engine::SoundOf, pitch: i32) -> Beat {
    Beat::Flow(FlowBeat::WaitSound { of, pitch })
}

/// A place in the scene, by that name.
/// A line of the film starts here, so ending (see `time::plan`).
pub fn film_line(number: i32, end: engine::LineEnd) -> Beat {
    Beat::Flow(FlowBeat::Line { number, end })
}

pub fn label(name: impl Into<String>) -> Beat {
    Beat::Flow(FlowBeat::Label(name.into()))
}

/// The scene goes on at that label.
pub fn goto(name: impl Into<String>) -> Beat {
    Beat::Flow(FlowBeat::Goto(name.into()))
}

/// Back past where the passage was gone to.
pub fn back() -> Beat {
    Beat::Flow(FlowBeat::Return)
}

/// The passage after that label played meanwhile, along with the rest.
pub fn parallel(name: impl Into<String>) -> Beat {
    Beat::Flow(FlowBeat::Parallel(Some(name.into())))
}

/// What follows plays only if none of the tests before held.
pub fn otherwise() -> Beat {
    Beat::Flow(FlowBeat::Else)
}

/// The scene waits until the view stops moving, or the light changing.
pub fn wait_cam() -> Beat {
    Beat::Flow(FlowBeat::WaitCam)
}

/// A time written in a line (`1.5s`, `6i`, `12`) in frames, as the engine
/// reads it; none if it would not.
pub fn frames_read(time: &str) -> Option<u32> {
    match (time.strip_suffix('s'), time.strip_suffix('i')) {
        (Some(seconds), _) => Some((seconds.parse::<f32>().ok()? * 60.0).round().max(0.0) as u32),
        (_, Some(frames)) => frames.parse().ok(),
        _ => time.parse().ok(),
    }
}

/// The scene waits for someone to be that near a point (found as it plays, maybe).
pub fn wait_near(who: Who, at: Point, radius: f32) -> Beat {
    Beat::Flow(FlowBeat::WaitNear { who, at, radius })
}
