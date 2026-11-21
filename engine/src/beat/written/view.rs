//! How a `ViewBeat` is written as its line (read by `beat::view`).

use crate::beat::{Cam, ViewBeat};
#[cfg(test)]
use crate::stage::Ease;
use crate::vocabulary::Vocabulary;

fn cam(on: &Cam) -> Vec<String> {
    match on {
        Cam::At(at) => vec!["@".to_string(), at.x.to_string(), at.y.to_string()],
        Cam::Who(who) => vec![who.to_string()],
    }
}

impl ViewBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let mut line = vec![];
        let word = match self {
            ViewBeat::Camera(on) => {
                // (No one: a word that names no one.)
                line.extend(on.as_ref().map_or_else(|| vec!["off".to_string()], cam));
                "camera"
            }
            ViewBeat::Ease(ease) => {
                line.push(ease.to_string());
                "ease"
            }
            ViewBeat::Pan { to, frames } => {
                line.extend(cam(to));
                line.push(frames.to_string());
                "pan"
            }
            ViewBeat::PanStays { to, frames } => {
                line.extend(cam(to));
                line.extend([frames.to_string(), "fixed".to_string()]);
                "pan"
            }
            ViewBeat::Frame { who, at, frames, most } => {
                if who.is_empty() && at.is_empty() {
                    return None;
                }
                line.push(frames.to_string());
                line.extend(who.iter().map(|who| who.to_string()));
                line.extend(at.iter().map(|at| format!("@{},{}", at.x, at.y)));
                line.extend(most.map(|most| format!("max:{most}")));
                "frame"
            }
            ViewBeat::KeepFrame(who) => {
                match who {
                    // (No one to keep is read as none.)
                    Some(who) if who.is_empty() => return None,
                    Some(who) => line.extend(who.iter().map(|who| who.to_string())),
                    None => line.push("off".to_string()),
                }
                "keepframe"
            }
            ViewBeat::Glance { to, frames, hold } => {
                line.extend(cam(to));
                line.extend([frames.to_string(), hold.to_string()]);
                "glance"
            }
            ViewBeat::Iris { open, frames, on } => {
                line.extend([if *open { "in" } else { "out" }.to_string(), frames.to_string()]);
                line.extend(on.map(|who| who.to_string()));
                "iris"
            }
            ViewBeat::Tempo(pace) => {
                // (Read within those bounds.)
                if !(0.1..=4.0).contains(pace) {
                    return None;
                }
                line.push(pace.to_string());
                "tempo"
            }
            ViewBeat::Spotlight(lit) => {
                match lit {
                    Some(lit) => {
                        line.extend(cam(&lit.on));
                        line.push(lit.radius.to_string());
                    }
                    None => line.push("off".to_string()),
                }
                "spotlight"
            }
            ViewBeat::SpotlightAdd(on, radius) => {
                line.push("add".to_string());
                line.extend(cam(on));
                line.push(radius.to_string());
                "spotlight"
            }
            _ => return None,
        };
        line.insert(0, word.to_string());
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frames;
    use crate::beat::Beat;
    use crate::stage::Lit;
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;
    use glam::Vec2;

    #[test]
    fn every_view_beat_written_is_read_back_the_same() {
        let at = Cam::at(Vec2::new(12.5, -3.0));
        let pikachu = Cam::Who(Who::Monster(25));
        let beats = [
            ViewBeat::Camera(None),
            ViewBeat::Camera(Some(at.clone())),
            ViewBeat::Camera(Some(pikachu.clone())),
            ViewBeat::Ease(Ease::Smooth),
            ViewBeat::Ease(Ease::Linear),
            ViewBeat::Ease(Ease::Out(crate::stage::EaseFamily::Quad)),
            ViewBeat::Ease(Ease::Bezier([0.4, 0.0, 0.2, 1.0])),
            ViewBeat::Pan { to: at.clone(), frames: Frames(30.0) },
            ViewBeat::Pan { to: pikachu.clone(), frames: Frames(45.5) },
            ViewBeat::PanStays { to: at.clone(), frames: Frames(30.0) },
            ViewBeat::PanStays { to: pikachu.clone(), frames: Frames(20.0) },
            ViewBeat::BoxRoom(false),
            ViewBeat::Zoom { to: 2.0, frames: Frames(40.0) },
            ViewBeat::Frame { who: vec![Who::Character(0), Who::Target], at: vec![Vec2::new(12.0, 30.0)], frames: Frames(30.0), most: Some(0.5) },
            ViewBeat::Frame { who: vec![Who::Thing { object: 66, copy: 0 }], at: Vec::new(), frames: Frames(10.0), most: None },
            ViewBeat::KeepFrame(None),
            ViewBeat::KeepFrame(Some(vec![Who::Character(0), Who::Character(10)])),
            ViewBeat::Body { who: Who::Monster(130), size: Vec2::new(48.0, 32.0) },
            ViewBeat::Glance { to: at.clone(), frames: Frames(30.0), hold: 60.0 },
            ViewBeat::Glance { to: pikachu.clone(), frames: Frames(15.0), hold: 90.0 },
            ViewBeat::Shake { axis: 1, strength: 2.5, frames: Some(Frames(30.0)) },
            ViewBeat::Shake { axis: 2, strength: 1.0, frames: None },
            ViewBeat::Quake { strength: 3.0, frames: Frames(40.0) },
            ViewBeat::Iris { open: true, frames: Frames(40.0), on: Some(Who::Character(0)) },
            ViewBeat::Iris { open: false, frames: Frames(20.0), on: None },
            ViewBeat::Letterbox(true),
            ViewBeat::Tempo(0.5),
            ViewBeat::Spotlight(None),
            ViewBeat::Spotlight(Some(Lit { on: at.clone(), radius: 40.0 })),
            ViewBeat::Spotlight(Some(Lit { on: pikachu.clone(), radius: 25.0 })),
            ViewBeat::SpotlightAdd(at.clone(), 30.0),
            ViewBeat::SpotlightAdd(pikachu.clone(), 12.0),
            ViewBeat::KeepSpeaker(false),
        ];
        for beat in beats {
            let line = Beat::View(beat.clone()).fields(&SAMPLE).expect("written");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::View(beat)), "{line:?}");
        }
    }
}
