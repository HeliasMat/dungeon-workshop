//! What the frame shows wrong: a variant shown a way it is not painted in, a
//! big body cut by the edge, who the view left out.

use super::watch::*;
use super::*;

impl Watch {
    /// A variant shown for half a second a way it is not painted in: its old
    /// pictures show. (Aiming and firing are its own, painted every way.)
    pub(super) fn unpainted(&mut self, director: &Director, stage: &Stage, line: i32) {
        for (who, ways) in stage.painted() {
            let shown = stage.cast.get(&who).filter(|actor| actor.attributes & attribute::HIDDEN == 0 && actor.shown_opacity() > 0.1);
            let wrong = shown.is_some_and(|actor| !ways.contains(&actor.direction)) && !director.manner.held_gaze.contains(&who);
            let frames = self.unpainted.entry(who).or_insert(0);
            *frames = if wrong { *frames + 1 } else { 0 };
            if *frames == 30 {
                let way = shown.map_or(0, |actor| actor.direction);
                let painted: Vec<&str> = ways.iter().map(|way| way_named(*way)).collect();
                let text = told!(
                    "{} faces {}, but is only drawn facing {}: its ordinary picture shows → look {} {}",
                    who,
                    way_named(way),
                    painted.join(", "),
                    who,
                    *painted.first().unwrap_or(&"down")
                );
                self.tell(line, format!("painted {who} {line}"), text);
            }
        }
    }

    /// Someone there, or gone, from a frame to the next, in sight: with no
    /// fade, flash, vision or picture over the scene, no smoke about him, no
    /// fading of his own; not one who blinked as he fell (the game's way),
    /// nor one a picture takes the place of, there (a flight seen from the
    /// sky, landing); a scene's first second is its own.
    pub(super) fn pops(&mut self, stage: &Stage, centre: Vec2, line: i32) {
        const NEAR: f32 = 28.0;
        let light = &stage.light;
        let covered = light.fade.level > 0.05 || light.fade.level != light.fade.to || light.flash.colour[3] > 0.05 || light.cutaway.is_some() || light.grey;
        let early = self.frame < self.scene_began + 60;
        let zoom = stage.view.closeness();
        let now: HashMap<Who, Showing> = stage
            .cast
            .iter()
            .map(|(who, actor)| {
                let shown = if actor.attributes & attribute::HIDDEN != 0 { 0.0 } else { actor.shown_opacity() };
                (*who, Showing { shown, at: actor.position, blinks: actor.attributes & attribute::BLINKING != 0 })
            })
            .collect();
        let before = std::mem::take(&mut self.showing);
        let body = |who: &Who| matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. });
        // (What changed from all to nothing, or back, at once: a fade goes by halves.)
        let flipped = |who: &Who| {
            let was = before.get(who).map_or(0.0, |one| one.shown);
            let is = now.get(who).map_or(0.0, |one| one.shown);
            (was >= 0.9 && is <= 0.1) || (was <= 0.1 && is >= 0.9)
        };
        if !covered && !early {
            let changed: Vec<(Who, Vec2)> = now
                .iter()
                .map(|(who, one)| (*who, one.at))
                .chain(before.iter().filter(|(who, _)| !now.contains_key(who)).map(|(who, one)| (*who, one.at)))
                .filter(|(who, _)| flipped(who))
                .collect();
            for (who, at) in changed.iter().filter(|(who, _)| body(who)) {
                let comes = now.get(who).is_some_and(|one| one.shown >= 0.9);
                let blinked = before.get(who).is_some_and(|one| one.blinks);
                let swapped = changed.iter().any(|(other, there)| !body(other) && there.distance(*at) < NEAR * 1.5);
                let shown = (*at - centre) * zoom;
                let in_sight = shown.x.abs() < HALF.x - 8.0 && shown.y.abs() < HALF.y - 8.0;
                let smoke = light.particles.iter().any(|spark| spark.at.distance(*at) < NEAR || spark.at.distance(*at - Vec2::new(0.0, 16.0)) < NEAR);
                if in_sight && !smoke && !blinked && !swapped {
                    let text = match comes {
                        true => told!(
                            "{} appears all at once in the picture, with nothing to justify it, at {} → bring them in (enter, put), or appear (in smoke)",
                            *who,
                            point(*at)
                        ),
                        false => told!(
                            "{} disappears all at once from the picture, with nothing to justify it, at {} → take them out (exit … via …), or vanish (a fade)",
                            *who,
                            point(*at)
                        ),
                    };
                    self.tell(line, format!("pops {who} {line}"), text);
                }
            }
        }
        self.showing = now;
    }

    /// A big body mostly out of the frame, its edge cutting it, for a second.
    pub(super) fn cut_by_the_edge(&mut self, stage: &Stage, centre: Vec2, line: i32) {
        const BIG: Vec2 = Vec2::new(48.0, 56.0);
        let zoom = stage.view.closeness();
        let (low, high) = (centre - HALF / zoom, centre + HALF / zoom);
        let steady = stage.view.pan.is_none() && stage.view.zoom.left <= 0.0 && stage.light.fade.level < 0.5;
        for (who, body) in stage.view.bodies.iter().filter(|(_, body)| body.x >= BIG.x || body.y >= BIG.y) {
            let Some(actor) = stage.cast.get(who).filter(|actor| actor.attributes & attribute::HIDDEN == 0 && actor.shown_opacity() > 0.1) else {
                continue;
            };
            let (from, to) = (actor.position - Vec2::new(body.x / 2.0, body.y), actor.position + Vec2::new(body.x / 2.0, 0.0));
            let seen = (to.min(high) - from.max(low)).max(Vec2::ZERO);
            let share = seen.x * seen.y / (body.x * body.y).max(1.0);
            let frames = self.cut.entry(*who).or_insert(0);
            *frames = if steady && share > 0.0 && share < 0.5 { *frames + 1 } else { 0 };
            if *frames == 60 {
                let text = told!("only {} % of {} shows: the edge of the picture cuts them", (share * 100.0).round().to_string(), *who);
                self.tell(line, format!("cut {who} {}", self.scene), text);
            }
        }
    }

    /// Who the view keeps, or who fights, and the view has settled without.
    pub(super) fn left_out(&mut self, stage: &Stage, centre: Vec2, resting: &[(Who, Vec2)], line: i32, scene: usize) {
        // (A view still on its way has not left anyone out.)
        let steady = std::mem::replace(&mut self.looked, centre).distance(centre) < 1.0;
        if !steady || stage.view.pan.is_some() || stage.view.zoom.left > 0.0 || stage.light.fade.level > 0.5 {
            return;
        }
        let zoom = stage.view.closeness();
        let kept: Vec<(Who, Vec2)> = resting.iter().filter(|(who, _)| stage.view.framing.contains(who) || self.fighters.contains(who)).copied().collect();
        for (who, at) in kept {
            let shown = (at - centre) * zoom;
            if shown.x.abs() > HALF.x || shown.y.abs() > HALF.y {
                self.tell(line, format!("frame {who} {scene}"), told!("{} stays {}, though the shot or the fight keeps them", who, side(at, centre, zoom)));
            }
        }
    }
}
