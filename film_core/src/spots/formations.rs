//! Several standing in a shape: `row`, `column`, `triangle`, `face to face`,
//! walked to or stood in before the scene shows; `rank`, facing us.

use super::routes::{USUAL, found, gait, go, puff};
use super::{Goal, goal_of, xy};
use crate::emit;
use engine::{Formation as Shape, Spot, Who};

use crate::film::Film;
use crate::verbs::Line;
use crate::words::{WAYS, plain};

/// A line's formation: its kind, who, the words of where, what they look at.
pub struct Formation {
    kind: String,
    who: Vec<String>,
    place: Vec<String>,
    looks: Option<String>,
}

/// The shapes a formation may take, but `face to face`: `ring` round its
/// place, facing it; `lineup` a row facing us.
const SHAPES: [&str; 5] = ["row", "column", "triangle", "ring", "lineup"];
/// How far apart those of a `rank` stand.
const RANK_STEP: f32 = 34.0;

/// The formation a line says, if it says one, by its values: its kind (the
/// word of the line, or of its form: `put row a b c`), who, where, what
/// they look at.
pub fn shape_of(line: &Line) -> Option<Formation> {
    let args = &line.args;
    let face = args.has("face") && (line.verb == "face" || args.has("to"));
    let kind = SHAPES.iter().find(|kind| line.verb == **kind || args.has(kind)).map(|kind| kind.to_string()).or(face.then(|| "face".to_string()))?;
    let who = args.words("who").to_vec();
    (!who.is_empty()).then(|| Formation {
        kind,
        who,
        place: args.words("where").to_vec(),
        // (`look left`, `look piplup`: named `looks` in `form`, `to` in the others' forms.)
        looks: args.word("looks").or(args.word("to")).map(plain).filter(|way| way != "face" && way != "to"),
    })
}

/// They stand in it, as the verb says: put there, walked, run, come in, or
/// appearing in smoke.
pub fn stand_in(film: &mut Film, shape: Formation, verb: &str, line: &Line, pace: &str) {
    let number = line.number;
    let Formation { kind, who, place, looks } = shape;
    let wanted = match kind.as_str() {
        "triangle" => Some(3),
        "face" => Some(2),
        _ => None,
    };
    if wanted.is_some_and(|wanted| who.len() != wanted) || who.len() < 2 {
        let many = wanted.map_or("two or more".to_string(), |wanted| wanted.to_string());
        film.wrong(number, format!("{kind} of whom? {many}, named after the word"));
        return;
    }
    let round = match goal_of(film, &place, number) {
        Goal::Spot(spot) => spot,
        _ => film.middle_spot(),
    };
    let Some(codes) = who.iter().map(|one| film.coded(one)).collect::<Option<Vec<Who>>>() else {
        return;
    };
    let shape = match kind.as_str() {
        "triangle" => Shape::Triangle,
        "face" => Shape::Facing,
        "column" => Shape::Column,
        "ring" => Shape::Ring,
        // (`row` and `lineup`, the only shapes left: see `SHAPES`.)
        _ => Shape::Line,
    };
    let look_of = |at: usize| match &looks {
        Some(way) if WAYS.contains(&way.as_str()) => way.as_str(),
        Some(_) => "down",
        None if kind == "face" => ["right", "left"][at],
        None => "down",
    };
    let mut walked = false;
    for (at, one) in who.iter().enumerate() {
        let spot = Spot::Shape { kind: shape, who: codes.clone(), at, round: Box::new(round.clone()) };
        let to = found(film, one, spot, &[]);
        if verb == "appear" {
            puff(film, to.clone());
        }
        if verb == "appear" || (verb == "put" && !film.staging.started) {
            match engine::Direction::of(look_of(at)) {
                Some(way) => film.on_at(one, to, |who, point| emit::put_point(who, point, way)),
                None => film.not_a(look_of(at), "a direction"),
            }
        } else {
            film.arrives(one, gait(verb), pace, to, verb != "enter", None);
            walked = true;
        }
    }
    if walked && line.wait {
        film.beat(emit::wait_all());
    }
    if kind == "ring" && looks.is_none() && (line.wait || !walked) {
        // (Each facing the ring's middle.)
        let middle = found(film, &who[0], round, &["exact"]);
        for one in &who {
            film.on_at(one, middle.clone(), emit::look_at_point);
        }
        return;
    }
    if line.wait || !walked {
        for (at, one) in who.iter().enumerate() {
            match looks.as_ref().filter(|way| !WAYS.contains(&way.as_str())) {
                Some(other) if film.actor(other).is_some() => film.face(one, other),
                _ => match engine::Direction::of(look_of(at)) {
                    Some(way) => film.on(one, |who| emit::turn(who, way)),
                    None => film.not_a(look_of(at), "a direction"),
                },
            }
        }
    }
}

/// `row a b c [where] [look x]`: they walk there, or stand there before
/// the scene shows, as `verb` says.
pub fn formation(film: &mut Film, line: &Line, verb: &str) {
    if let Some(shape) = shape_of(line) {
        stand_in(film, shape, verb, line, &super::routes::pace_of(&line.args));
    }
}

/// In a row, facing whoever watches: the guild's apprentices at dawn.
pub fn on_rang(film: &mut Film, who: &[String]) {
    let (x, y) = film.middle;
    let middle_of = (who.len() as f32 - 1.0) / 2.0;
    for (at, one) in who.iter().enumerate() {
        let spot = Spot::Word(xy((x as f32 + (at as f32 - middle_of) * RANK_STEP) as i32, y));
        go(film, one, spot, "walk", false, USUAL, false);
    }
    film.beat(emit::wait_all());
    for one in who {
        film.on(one, |who| emit::turn(who, engine::Direction::Down));
    }
}
