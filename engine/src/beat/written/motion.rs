//! How a `MotionBeat` is written as its line (read by `beat::motion`).

use crate::beat::MotionBeat;
use crate::vocabulary::Vocabulary;
use crate::who::Who;

impl MotionBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let line = |verb: &str, rest: &[String]| std::iter::once(verb.to_string()).chain(rest.iter().cloned()).collect::<Vec<String>>();
        let codes = |who: &[Who]| who.iter().map(Who::to_string).collect::<Vec<String>>();
        let or = |who: &Option<Who>, none: &str| who.as_ref().map_or(none.to_string(), Who::to_string);
        let numbers = |numbers: &[f32]| numbers.iter().map(f32::to_string).collect::<Vec<String>>();
        Some(match self {
            MotionBeat::Walk { who, at, run } => line(if *run { "run" } else { "walk" }, &[who.to_string(), at.x.to_string(), at.y.to_string()]),
            MotionBeat::WalkTo { who, at, run } => line(if *run { "runto" } else { "walkto" }, &[who.to_string(), at.x.to_string(), at.y.to_string()]),
            // (A path is read with one point at least.)
            MotionBeat::Path { who, points, run } if !points.is_empty() => {
                let mut fields = line(if *run { "runpath" } else { "path" }, &[who.to_string()]);
                fields.extend(points.iter().flat_map(|point| [point.x.to_string(), point.y.to_string()]));
                fields
            }
            MotionBeat::Step { who, dx, dy, run } => line(if *run { "runstep" } else { "step" }, &[who.to_string(), dx.to_string(), dy.to_string()]),
            MotionBeat::Orbit { who, at, turns, frames } => {
                line("orbit", &[who.to_string(), at.x.to_string(), at.y.to_string(), turns.to_string(), frames.to_string()])
            }
            // (Round someone whose code is a number would be read as round a point.)
            MotionBeat::OrbitAround { who, other, turns, frames } if other.to_string().parse::<f32>().is_err() => {
                line("orbit", &[who.to_string(), other.to_string(), turns.to_string(), frames.to_string()])
            }
            // (No leader written as `-`, which is no one, so a gap may follow.)
            MotionBeat::Follow { who, leader, gap } => {
                let mut fields = line("follow", &[who.to_string(), or(leader, "-")]);
                fields.extend(gap.map(|gap| gap.to_string()));
                fields
            }
            // (Those read in a group are one at least.)
            MotionBeat::LineUp { who, at, gap } if !who.is_empty() => line("lineup", &[numbers(&[at.x, at.y, gap.x, gap.y]), codes(who)].concat()),
            MotionBeat::Ring { who, at, radius } if !who.is_empty() => line("ring", &[numbers(&[at.x, at.y, *radius]), codes(who)].concat()),
            // (No line any more is read as a gap of 0 and no one; a line, at least 4 apart, two in it.)
            MotionBeat::Train { gap, who } if who.is_empty() && *gap == 0.0 => line("train", &["off".to_string()]),
            MotionBeat::Train { gap, who } if who.len() >= 2 && *gap >= 4.0 => line("train", &[numbers(&[*gap]), codes(who)].concat()),
            MotionBeat::KeepOnly(who) if !who.is_empty() => line("keeponly", &codes(who)),
            MotionBeat::Mirror { who, model } => line("mirror", &[who.to_string(), or(model, "off")]),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Frames;
    use crate::beat::{Beat, MotionBeat};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;
    use glam::Vec2;

    #[test]
    fn every_motion_beat_written_is_read_back_the_same() {
        let (a, b) = (Who::Monster(25), Who::Character(0));
        let group = vec![a, b, Who::Another { species: 25, copy: 1 }];
        for beat in [
            MotionBeat::Go { who: a, at: glam::Vec2::new(10.0, -2.5).into(), frames: Frames(30.0), slide: false },
            MotionBeat::Go { who: a, at: glam::Vec2::new(10.0, -2.5).into(), frames: Frames(30.0), slide: true },
            MotionBeat::Walk { who: a, at: glam::Vec2::new(1.0, 2.0).into(), run: false },
            MotionBeat::Walk { who: a, at: glam::Vec2::new(1.0, 2.0).into(), run: true },
            MotionBeat::WalkTo { who: a, at: glam::Vec2::new(1.0, 2.0).into(), run: false },
            MotionBeat::WalkTo { who: a, at: glam::Vec2::new(1.0, 2.0).into(), run: true },
            MotionBeat::Path { who: a, points: vec![Vec2::new(1.0, 2.0), Vec2::new(3.5, 4.0)], run: false },
            MotionBeat::Path { who: a, points: vec![Vec2::new(1.0, 2.0)], run: true },
            MotionBeat::Step { who: a, dx: 16.0, dy: 0.0, run: false },
            MotionBeat::Step { who: a, dx: 16.0, dy: 0.0, run: true },
            MotionBeat::Speed { who: a, pace: 1.2 },
            MotionBeat::Orbit { who: a, at: glam::Vec2::new(5.0, 6.0).into(), turns: -2.0, frames: Frames(90.0) },
            MotionBeat::OrbitAround { who: a, other: Who::Monster(7), turns: 1.5, frames: Frames(60.0) },
            MotionBeat::Resume(a),
            MotionBeat::Follow { who: a, leader: Some(b), gap: Some(20.0) },
            MotionBeat::Follow { who: a, leader: Some(b), gap: None },
            MotionBeat::Follow { who: a, leader: None, gap: None },
            MotionBeat::Follow { who: a, leader: None, gap: Some(8.0) },
            MotionBeat::Approach { who: a, other: b, frames: Frames(40.0) },
            MotionBeat::Flee { who: a, from: b, distance: 48.0 },
            MotionBeat::Chase { who: a, other: b },
            MotionBeat::LineUp { who: group.clone(), at: Vec2::new(1.0, 2.0), gap: Vec2::new(24.0, 0.0) },
            MotionBeat::Ring { who: group.clone(), at: Vec2::new(1.0, 2.0), radius: 30.0 },
            MotionBeat::Train { gap: 0.0, who: Vec::new() },
            MotionBeat::Train { gap: 12.0, who: group.clone() },
            MotionBeat::KeepOnly(group),
            MotionBeat::Mirror { who: a, model: Some(b) },
            MotionBeat::Mirror { who: a, model: None },
            MotionBeat::Wander { who: a, radius: 24.0 },
            MotionBeat::Spacing(true),
            MotionBeat::Spacing(false),
        ] {
            let fields = Beat::Motion(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(Beat::Motion(beat)), "{fields:?}");
        }
    }
}
