//! The bodies at rest and walking: on the scenery, on one another, stuck,
//! among fighters, hidden by a crowd.

use super::watch::*;
use super::*;

impl Watch {
    pub(super) fn bodies(&mut self, stage: &Stage, centre: Vec2, line: i32, scene: usize) {
        // (Who has left the stage is forgotten: back, it walks anew.)
        self.progress.retain(|who, _| stage.cast.contains_key(who));
        self.still.retain(|who, _| stage.cast.contains_key(who));
        let walls = &stage.play.place_walls;
        // (With no collisions, walls and bodies are no one's concern.)
        let collides = !stage.no_collisions;
        let on_scenery = |at: Vec2| walls.iter().any(|wall| wall.holds(at, 0.0)) || stage.place.ground.as_ref().is_some_and(|ground| ground.blocked(at, 0));
        let fighting = !self.fighters.is_empty();
        let bodies = stage.cast.iter().filter(|(who, actor)| {
            matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. }) && actor.attributes & attribute::HIDDEN == 0 && actor.shown_opacity() > 0.1
        });
        let mut resting = Vec::new();
        for (who, actor) in bodies {
            // (Still since he set off, if he walks: not the rest before.)
            let setting_off = actor.walks() && !self.progress.contains_key(who);
            let held = self.still.get(who).filter(|(at, _)| at.distance(actor.position) < 1.0 && !setting_off).map_or(0, |(_, frames)| frames + LOOK_EVERY);
            self.still.insert(*who, (actor.position, held));
            // (Who rides, or is carried, is on its mount, not the ground.)
            // (Nor who swims by nature, in the water.)
            let grounded = actor.shown_height() < 8.0 && !stage.place.flying.contains(who) && !stage.place.swimming.contains(who) && !actor.carried();
            if held >= AT_REST && !actor.busy() && grounded {
                resting.push((*who, actor.position));
            }
            // (Going no nearer where it goes, if it shuffles on the spot.)
            let left = stage.left_to_walk(*who);
            let (best, looks) = self.progress.get(who).copied().filter(|_| actor.walks()).unwrap_or((f32::MAX, 0));
            let (best, looks) = if left < best - 4.0 { (left, 0) } else { (best, looks + 1) };
            match actor.walks() {
                true => _ = self.progress.insert(*who, (best, looks)),
                false => _ = self.progress.remove(who),
            }
            if actor.walks() && grounded && !fighting && collides {
                // (Not stuck where it is going already: an approach that ends on
                // its own spot.)
                let there = actor.position.distance(actor.goal()) < 6.0;
                // (Nor one who follows in a file: it waits for the gap.)
                let following = stage.train.as_ref().is_some_and(|train| train.who.iter().skip(1).any(|one| one == who));
                if (held >= STUCK || looks * LOOK_EVERY >= SHUFFLING) && !there && !following {
                    let short = actor.position.distance(actor.goal()).round();
                    let text = told!(
                        "{} walks on the spot, stuck {} pixels from where they are going, {} → another spot, or a path (walk … via …)",
                        *who,
                        short.to_string(),
                        whereabouts(stage, actor.position)
                    );
                    self.tell(line, format!("stuck {who} {scene}"), text);
                } else if on_scenery(actor.position) && !actor.habits.vanishes && !stage.place.entering.contains_key(who) {
                    let text = told!("{} walks through the backdrop, {} → a path around it (walk … via …)", *who, whereabouts(stage, actor.position));
                    self.tell(line, format!("through {who} {scene}"), text);
                }
            }
        }
        for (who, at) in &resting {
            if collides && on_scenery(*at) {
                let text =
                    told!("{} stands inside the backdrop (a wall, a tree), {} → a free tile (scene_player ground <place>)", *who, whereabouts(stage, *at));
                self.tell(line, format!("wall {who} {}", point(*at)), text);
            }
        }
        // (A thing a film put down; those of the place stand where they stand.)
        let lying = stage.things().into_iter().filter(|thing| !thing.name.is_empty() && thing.owner.is_none() && thing.height < 2.0);
        for thing in lying.filter(|thing| collides && on_scenery(thing.place)).collect::<Vec<_>>() {
            self.tell(
                line,
                format!("thing {} {}", thing.id, point(thing.place)),
                format!("{} is put inside the backdrop, at {}", thing.name, point(thing.place)),
            );
        }
        // (A line that falls on the ground, not in the water.)
        for (who, to) in &stage.lines {
            let floating = stage
                .cast
                .get(to)
                .filter(|actor| !actor.walks() && actor.shown_height() < 2.0 && !matches!(to, Who::Character(_) | Who::Monster(_) | Who::Another { .. }));
            // (Nor one on open water a place does not mark: cast at sea, `flies`.)
            if let Some(actor) = floating.filter(|actor| !on_scenery(actor.position) && !stage.place.flying.contains(to)) {
                self.tell(
                    line,
                    format!("line {who} {}", point(actor.position)),
                    told!("the line of {} falls on the ground at {}: not in the water", *who, point(actor.position)),
                );
            }
        }
        self.among_fighters(stage, &resting, line, scene);
        self.hidden_by_the_crowd(&resting, line, scene);
        self.left_out(stage, centre, &resting, line, scene);
        if resting.len() > MOST_PAIRED || !collides {
            return;
        }
        for (place, (who, at)) in resting.iter().enumerate() {
            for (other, there) in &resting[place + 1..] {
                // (Two who fight meet in a blow: that is no one in the wrong place.)
                let blow = self.fighters.contains(who) && self.fighters.contains(other);
                if !blow && at.distance(*there) < ON_TOP {
                    self.tell(
                        line,
                        format!("on {who} {other} {}", point(*at)),
                        told!("{} and {} are on one another, {} → move them apart (put {} left_of {})", *who, *other, whereabouts(stage, *at), *who, *other),
                    );
                }
            }
        }
    }

    /// One of a crowd standing just before one who is not, drawn over it.
    fn hidden_by_the_crowd(&mut self, resting: &[(Who, Vec2)], line: i32, scene: usize) {
        if self.crowd.is_empty() {
            return;
        }
        let (crowd, principal): (Vec<&(Who, Vec2)>, Vec<_>) = resting.iter().partition(|(who, _)| self.crowd.contains(who));
        for (who, at) in principal {
            if let Some((other, _)) = crowd.iter().find(|(_, there)| (there.x - at.x).abs() < 14.0 && there.y > at.y && there.y - at.y < 20.0) {
                self.tell(
                    line,
                    format!("crowd {who} {scene}"),
                    told!("{} of the crowd is just in front of {} and hides them, at {}", *other, *who, point(*at)),
                );
            }
        }
    }

    /// Those who only watch, standing on the field between the fighters.
    fn among_fighters(&mut self, stage: &Stage, resting: &[(Who, Vec2)], line: i32, scene: usize) {
        let places: Vec<Vec2> = self.fighters.iter().filter_map(|who| stage.cast.get(who)).map(|actor| actor.position).collect();
        let Some(first) = places.first().copied() else {
            return;
        };
        let (low, high) = places.iter().fold((first, first), |(low, high), at| (low.min(*at), high.max(*at)));
        let (low, high) = (low - Vec2::splat(FIELD), high + Vec2::splat(FIELD));
        let among: Vec<&Who> =
            resting.iter().filter(|(who, at)| !self.fighters.contains(who) && at.cmpge(low).all() && at.cmple(high).all()).map(|(who, _)| who).collect();
        if let Some(who) = among.first() {
            let (count, one) = (among.len(), **who);
            let text = match count {
                1 => told!("{} does not fight, and stands on the fight's field", one),
                _ => told!("{} Pokémon who do not fight are on the fight's field, {} among them", count.to_string(), one),
            };
            self.tell(line, format!("field {scene}"), text);
        }
    }
}
