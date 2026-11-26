//! The camera: Sky's performer 0, which it follows; frames, pans, shakes.

use engine::{Beat, ScreenBeat, ViewBeat};

use super::{FEW, PAN, Writer, pan};
use crate::notes::{Fate, Kind};

impl Writer<'_> {
    /// A view beat: where the camera goes, how the screen shakes.
    pub(super) fn view(&mut self, number: i32, word: Option<&str>, beat: &ViewBeat) {
        match beat {
            // A slow motion: Sky's scripts have none; the finishing blow plays at its pace.
            ViewBeat::Tempo(_) if word == Some("finish") => {}
            // The engine's evolution (slowed, shining, growing): Sky's white flash instead (`body`, `evolve`).
            ViewBeat::Tempo(_) if matches!(word, Some("evolve" | "transform")) => {}
            // The camera: Sky's performer 0, which the camera follows.
            ViewBeat::Frame { who, at, frames, .. } => {
                let mut points: Vec<(f32, f32)> = who.iter().filter_map(|one| self.at.get(one).copied()).collect();
                points.extend(at.iter().map(|point| (point.x, point.y)));
                if points.is_empty() {
                    self.problem(number, Kind::Logic, Fate::LeftOut, "where to frame is not known before the film plays", "left out");
                    return;
                }
                // A frame of a few: those are never left out of a crowded scene.
                if who.len() <= FEW {
                    self.framed.extend(who.iter().copied());
                }
                let count = points.len() as f32;
                let middle = (points.iter().map(|point| point.0).sum::<f32>() / count, points.iter().map(|point| point.1).sum::<f32>() / count);
                match pan(&points).filter(|_| self.started) {
                    // Wider (or taller) than the screen, and Sky has no zoom: a slow pan
                    // across them, from one end to the other, then back to their middle.
                    Some((from, to)) => {
                        self.camera_to(from, frames.0);
                        self.line("WaitExecutePerformer(0);");
                        let across = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
                        self.camera_to(to, across / PAN);
                        self.line("WaitExecutePerformer(0);");
                        let back = ((middle.0 - to.0).powi(2) + (middle.1 - to.1).powi(2)).sqrt();
                        self.camera_to(middle, back / PAN);
                        self.note(
                            number,
                            Kind::Camera,
                            Fate::Changed,
                            "the frame is wider than the screen (Sky has no zoom)",
                            format!("a pan across it ({across:.0} pixels)"),
                        );
                    }
                    None => self.camera_to(middle, frames.0),
                }
            }
            ViewBeat::Pan { to, frames } | ViewBeat::PanStays { to, frames } => match self.cam(to) {
                Some(point) => self.camera_to(point, frames.0),
                None => self.problem(number, Kind::Logic, Fate::LeftOut, "where the camera goes is not known before the film plays", "left out"),
            },
            ViewBeat::Camera(cam) => {
                let to = cam.as_ref().and_then(|cam| self.cam(cam)).or(self.middle);
                if let Some(to) = to {
                    self.camera_to(to, 0.0);
                }
            }
            // Sky's screen shakes (`camera_SetEffect`), soft and hard.
            ViewBeat::Shake { strength, .. } => {
                let shake = if *strength <= 0.0 {
                    "0, 0, 0"
                } else if *strength <= 2.0 {
                    "2, 1, 1.0"
                } else {
                    "2, 2, 3.0"
                };
                self.line(&format!("camera_SetEffect({shake});"));
            }
            ViewBeat::Quake { frames, .. } => {
                self.line("camera_SetEffect(2, 2, 3.0);");
                self.line(&format!("Wait({});", frames.0.round() as i32));
                self.line("camera_SetEffect(0, 0, 0);");
            }
            // (An iris: Sky fades, out as it closes.)
            ViewBeat::Iris { open, frames, .. } => self.screen(number, word, &ScreenBeat::Fade { out: !*open, white: false, frames: *frames }),
            other => self.unexported(number, word, &Beat::View(other.clone())),
        }
    }
}
