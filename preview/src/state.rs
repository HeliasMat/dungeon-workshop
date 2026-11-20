//! Who is where at a moment of a film: each one on stage (where they stand,
//! which way they face, their pose, their HP), the camera, the objects. What
//! `scene_player state` prints and the Workshop's State panel shows.

use engine::{Cam, Who, attribute};
use glam::Vec2;

use crate::film::Film;

/// The ways someone faces, 1 to 8.
pub const FACING: [&str; 8] = ["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"];

/// Someone on stage at a moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub who: Who,
    pub name: String,
    /// Where their feet are in the place.
    pub at: Vec2,
    pub faces: &'static str,
    /// Where they walk to (and whether they run).
    pub goes: Option<(Vec2, bool)>,
    pub pose: Option<&'static str>,
    /// How high above the ground, when it shows (8 pixels or more).
    pub above: Option<f32>,
    pub hidden: bool,
    /// Where the line tied to them goes.
    pub tied_to: Option<Vec2>,
    pub statuses: Vec<&'static str>,
    /// Their bar of HP, from 0 to 1.
    pub hp: Option<f32>,
    pub out_of_frame: bool,
}

/// An object on stage at a moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub name: String,
    pub at: Vec2,
    pub held_by: Option<String>,
    pub above: Option<f32>,
    /// The image of its strip shown (from 1).
    pub image: Option<usize>,
    pub takeable: bool,
    pub shines: bool,
}

/// The camera at a moment: the point at the middle of the view, how close,
/// who it follows, who it keeps in frame, whether it glides.
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub centre: Vec2,
    pub zoom: f32,
    pub follows: Option<String>,
    pub keeps: Vec<String>,
    pub glides: bool,
}

/// A moment of a film: its camera, its bars, the line in the box, who is on
/// stage (by name), the objects a film put there (by name).
#[derive(Clone, Debug, PartialEq)]
pub struct Moment {
    pub camera: Camera,
    pub bars: Vec<String>,
    pub said: Option<String>,
    pub bodies: Vec<Body>,
    pub objects: Vec<Object>,
}

/// The moment the film stands at; `name` names each one as the film does.
pub fn moment(film: &Film, name: &dyn Fn(Who) -> String) -> Moment {
    let play = film.play();
    let stage = &play.sim.stage;
    let view = film.view();
    let camera = Camera {
        centre: view.centre,
        zoom: view.close,
        follows: match stage.view.camera {
            Some(Cam::Who(who)) => Some(name(who)),
            Some(Cam::At(_)) | None => None,
        },
        keeps: stage.view.framing.iter().map(|who| name(*who)).collect(),
        glides: stage.view.pan.is_some(),
    };
    let mut bodies: Vec<Body> = stage
        .cast
        .iter()
        .filter(|(who, _)| matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. }))
        .map(|(who, actor)| {
            let shown = view.on_screen(actor.position);
            Body {
                who: *who,
                name: name(*who),
                at: actor.position,
                faces: FACING[(actor.direction.max(1) as usize - 1) % 8],
                goes: actor.walks().then(|| (actor.goal(), actor.runs())),
                pose: play.sim.pose(*who),
                above: (actor.shown_height() >= 8.0).then(|| actor.shown_height()),
                hidden: actor.attributes & attribute::HIDDEN != 0 || actor.shown_opacity() <= 0.1,
                tied_to: stage.lines.get(who).and_then(|to| stage.cast.get(to)).map(|to| to.position),
                statuses: stage.emotes.statuses_of(*who).to_vec(),
                hp: actor.bar,
                out_of_frame: shown.x < 0.0 || shown.y < 0.0 || shown.x > view.screen.x || shown.y > view.screen.y,
            }
        })
        .collect();
    bodies.sort_by(|a, b| a.name.cmp(&b.name));
    // (Those a film put; a place's own, with no name, left out.)
    let mut objects: Vec<Object> = stage
        .things()
        .into_iter()
        .filter(|thing| !thing.name.is_empty())
        .map(|thing| Object {
            held_by: thing.owner.map(name),
            above: (thing.owner.is_none() && thing.height >= 2.0).then_some(thing.height),
            image: thing.frame.map(|step| step + 1),
            name: thing.name,
            at: thing.place,
            takeable: thing.takeable,
            shines: thing.shines,
        })
        .collect();
    objects.sort_by(|a, b| a.name.cmp(&b.name));
    Moment {
        camera,
        bars: stage.words.hud.iter().flatten().map(|hud| play.sim.director.fill(hud)).collect(),
        said: film.page().map(stage_view::ui::strip_tags),
        bodies,
        objects,
    }
}

/// A point, rounded.
fn point(at: Vec2) -> String {
    format!("({}, {})", at.x.round(), at.y.round())
}

impl Body {
    /// What is told of them after where they stand, in short words.
    pub fn details(&self) -> Vec<String> {
        let mut said = vec![format!("faces {}", self.faces)];
        if let Some((to, runs)) = self.goes {
            said.push(format!("{} to {}", if runs { "runs" } else { "walks" }, point(to)));
        }
        said.extend(self.pose.map(|name| format!("pose {name}")));
        said.extend(self.above.map(|above| format!("{} above the ground", above.round())));
        if self.hidden {
            said.push("hidden".to_string());
        }
        said.extend(self.tied_to.map(|to| format!("its line goes to {}", point(to))));
        said.extend(self.statuses.iter().map(|status| format!("status {status}")));
        said.extend(self.hp.map(|bar| format!("HP {}%", (bar * 100.0).round())));
        if self.out_of_frame {
            said.push("out of frame".to_string());
        }
        said
    }

    /// Them, as a line.
    pub fn told(&self) -> String {
        std::iter::once(format!("{} {}", self.name, point(self.at))).chain(self.details()).collect::<Vec<String>>().join(", ")
    }
}

impl Object {
    /// What is told of it after where it is, in short words.
    pub fn details(&self) -> Vec<String> {
        let mut said = Vec::new();
        said.extend(self.held_by.as_ref().map(|owner| format!("held by {owner}")));
        said.extend(self.above.map(|above| format!("{} above the ground", above.round())));
        said.extend(self.image.map(|image| format!("image {image}")));
        if self.takeable {
            said.push("takeable".to_string());
        }
        if self.shines {
            said.push("shines".to_string());
        }
        said
    }

    /// It, as a line.
    pub fn told(&self) -> String {
        std::iter::once(format!("object {} {}", self.name, point(self.at))).chain(self.details()).collect::<Vec<String>>().join(", ")
    }
}

impl Camera {
    /// It, as a line.
    pub fn told(&self) -> String {
        let goes = self.follows.as_ref().map_or(String::new(), |who| format!(", follows {who}"));
        let kept = if self.keeps.is_empty() { String::new() } else { format!(", keeps {}", self.keeps.join(" ")) };
        format!("view ({}, {}), zoom {:.2}{goes}{kept}{}", self.centre.x.round(), self.centre.y.round(), self.zoom, if self.glides { ", glides" } else { "" })
    }
}
