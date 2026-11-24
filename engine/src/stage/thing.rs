//! Things on the stage: the game's items, the scenes' props, pictures of a
//! film's own. One idea for all three ([`Thing`]): each moves as anyone of the
//! cast does; here is what it is, whose it is, and how it goes from hand to
//! hand. A story of the game has its things told the same way.

use super::*;

/// How high over someone's feet what it carries over its head shows, and what
/// is in its hands; how far before it; how far apart two things in its hands.
const OVER_HEAD: f32 = 22.0;
const IN_HANDS: f32 = 9.0;
const BEFORE: f32 = 11.0;
const APART: f32 = 7.0;
/// How far below its hands what it holds is drawn from: held by its middle.
const HELD_BELOW: f32 = 6.0;

/// How far over its head one sits on someone's head, as a film seats him.
const ON_HEAD: f32 = 4.0;
/// Seen from above at a slant, how much of a step back along a back shows
/// up or down the screen.
const DEPTH_ON_BACK: f32 = 0.6;
/// Facing us, how much higher one sits behind its neck, to be seen over it.
pub(super) const BEHIND_THE_NECK: f32 = 3.0;
/// The frames a thing given shows over the giver; how high it flies.
const SHOWN_OVER: f32 = 16.0;
const HANDED_HEIGHT: f32 = 24.0;

/// How far before someone a line falls, unless told; how high it starts
/// from, how high it flies, and in how many frames.
const CAST_FAR: f32 = 60.0;
const CAST_FROM: f32 = 18.0;
const CAST_HEIGHT: f32 = 34.0;
pub const CAST_FRAMES: f32 = 30.0;

/// Where a thing's picture comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picture {
    Item(u16),
    /// A prop of the scenes, by its object.
    Prop(u16),
    /// A picture of a film's own, by its number there.
    Drawn(u16),
}

/// A thing of the stage as it is now.
#[derive(Clone, Debug, PartialEq)]
pub struct Thing {
    pub id: Who,
    /// As it is told; empty for one only the game's place has.
    pub name: String,
    pub picture: Picture,
    pub place: Vec2,
    pub height: f32,
    pub depth: f32,
    /// The step it is held on, if it is (none: it plays by itself).
    pub frame: Option<usize>,
    pub owner: Option<Who>,
    pub takeable: bool,
    pub shines: bool,
}

/// What is told of a thing, past what it has as anyone of the cast.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Marks {
    pub(crate) name: String,
    pub(crate) frame: Option<usize>,
    pub(crate) takeable: bool,
    pub(crate) shines: bool,
}

/// A thing on its way to someone's hands: over the giver for `wait` more
/// frames, then flying for `frames`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Handing {
    pub(crate) what: Who,
    from: Who,
    to: Who,
    wait: f32,
    frames: f32,
    flying: bool,
}

impl Stage {
    /// A thing as it is now, if there is one by that id.
    pub fn thing(&self, id: Who) -> Option<Thing> {
        let actor = self.cast.get(&id)?;
        let picture = match id {
            Who::Item { item, .. } => Picture::Item(item),
            Who::Thing { object, .. } => Picture::Prop(object),
            Who::Drawn { picture, .. } => Picture::Drawn(picture),
            _ => return None,
        };
        let marks = self.marks.get(&id).cloned().unwrap_or_default();
        Some(Thing {
            id,
            name: marks.name,
            picture,
            place: actor.position,
            height: actor.shown_height(),
            depth: actor.looks.depth,
            frame: marks.frame,
            owner: actor.habits.carried_by,
            takeable: marks.takeable,
            shines: marks.shines,
        })
    }

    /// All those on the stage.
    pub fn things(&self) -> Vec<Thing> {
        self.cast.keys().filter_map(|id| self.thing(*id)).collect()
    }

    /// Changes what is told of a thing.
    pub(super) fn marked(&mut self, who: Who, change: impl FnOnce(&mut Marks)) {
        if who.is_thing() {
            change(self.marks.entry(who).or_default());
        }
    }

    /// The step a thing is held on.
    pub(super) fn framed(&mut self, who: Who, step: Option<usize>) {
        self.marked(who, |marks| marks.frame = step);
    }

    /// A thing that shows over the giver, flies, and stays in the other's hands.
    pub(super) fn hand_over(&mut self, what: Who, from: Who, to: Who, frames: f32) {
        let start = self.cast.get(&from).map_or(Vec2::ZERO, |actor| actor.position);
        let thing = self.cast.entry(what).or_insert_with(|| Actor::at(start, 1));
        (thing.habits.carried_by, thing.habits.held) = (Some(from), false);
        self.handing.retain(|handing| handing.what != what);
        self.handing.push(Handing { what, from, to, wait: SHOWN_OVER, frames: frames.max(1.0), flying: false });
    }

    /// A line goes to something; told where, that thing flies there in an
    /// arc from the rod: before it by itself, if it is thrown to no point.
    pub(super) fn fish_line(&mut self, who: Who, to: Option<Who>, at: Option<Vec2>, cast: bool) {
        let Some(to) = to else {
            self.lines.remove(&who);
            return;
        };
        self.lines.insert(who, to);
        // (A catch: tied where it comes out of the water, not thrown.)
        if !cast {
            return;
        }
        let Some(from) = self.cast.get(&who).map(|actor| (actor.position, actor.direction)) else {
            return;
        };
        let there = at.unwrap_or(from.0 + facing(from.1) * CAST_FAR);
        self.with(to, |actor| {
            actor.position = from.0;
            actor.go(there, CAST_FRAMES, true);
            (actor.air.height, actor.air.arc) = (CAST_FROM, Some((CAST_HEIGHT, CAST_FRAMES)));
        });
    }

    /// Those on their way go on.
    pub(super) fn pass_handing(&mut self, frames: f32) {
        for mut handing in std::mem::take(&mut self.handing) {
            handing.wait -= frames;
            let there = self.cast.get(&handing.to).map(|actor| actor.position);
            match (there, handing.flying) {
                // (Who it was for has left: it falls where it is.)
                (None, _) => self.with(handing.what, |actor| (actor.habits.carried_by, actor.air.height) = (None, 0.0)),
                (Some(there), false) if handing.wait <= 0.0 => {
                    self.with(handing.what, |actor| {
                        actor.habits.carried_by = None;
                        actor.go(there, handing.frames, true);
                        (actor.air.height, actor.air.arc) = (OVER_HEAD, Some((HANDED_HEIGHT, handing.frames)));
                    });
                    self.handing.push(Handing { flying: true, ..handing });
                }
                (Some(_), true) if self.cast.get(&handing.what).is_some_and(|actor| actor.way.left <= 0.0) => self.handed(handing),
                _ => self.handing.push(handing),
            }
        }
    }

    /// In the other's hands.
    fn handed(&mut self, handing: Handing) {
        self.with(handing.what, |actor| (actor.habits.carried_by, actor.habits.held) = (Some(handing.to), true));
    }

    /// All those on their way get there.
    pub(super) fn hand_over_at_once(&mut self) {
        for handing in std::mem::take(&mut self.handing) {
            self.handed(handing);
        }
    }

    /// What is carried goes with who carries it, up and down with it, shown
    /// and hidden with it: over its head, or in its hands, on the side it
    /// faces (behind it when it looks away), several side by side; who
    /// rides, on its back, just before it.
    pub(super) fn carried_along(&mut self) {
        let carried: Vec<(Who, Who)> = self.cast.iter().filter_map(|(what, actor)| Some((*what, actor.habits.carried_by?))).collect();
        let mut in_hands: HashMap<Who, usize> = HashMap::new();
        for (what, by) in carried {
            let Some(owner) = self.cast.get(&by) else {
                continue;
            };
            // (On one sunk in the water, as low as it is.)
            let (at, direction, lift) = (owner.shown_at(), owner.direction, owner.shown_height() - owner.sunk);
            // (A mount that goes, or flies, turns its rider with it.)
            let turns = owner.way.left > 0.0 || owner.air.height > 0.0;
            let (hidden, opacity) = (owner.attributes & attribute::HIDDEN, owner.looks.opacity);
            // (Its points as it is drawn: turned round with it, grown with it.)
            let (flip, scale) = (if owner.flipped { -1.0 } else { 1.0 }, owner.looks.scale);
            let body = owner.body.map(|body| Body { now: body.now.map(|at| at * Vec2::new(flip, 1.0) * scale), standing: body.standing.map(|at| at * scale) });
            let slot = in_hands.entry(by).or_insert(0);
            let held = self.cast.get(&what).is_some_and(|actor| actor.habits.held);
            let beside = if held { *slot as f32 } else { 0.0 };
            *slot += usize::from(held);
            self.with(what, |actor| {
                let facing = facing(direction);
                let (there, high) = match (actor.habits.rides, actor.habits.held) {
                    // (On its back, so far behind its middle, on whichever side it
                    // faces. Facing us, its back is behind its
                    // neck: drawn behind it, higher on it; facing away, before it.)
                    (Some((back, behind)), _) => {
                        // (Turned with it as it goes, all the while it flies, and when it
                        // turns on the spot; else, still on the ground or the water, he
                        // looks where he will: he fishes off its back any way.)
                        let turned = actor.habits.mount_faced.is_some_and(|faced| faced != direction);
                        if turns || turned {
                            actor.direction = direction;
                        }
                        actor.habits.mount_faced = Some(direction);
                        let away = -facing * behind;
                        // (Its seat, its head or its back, whichever is the nearer to
                        // how high he sits, rises and sinks as it moves.)
                        let seat = body.map_or(Vec2::ZERO, |body| {
                            let off = |point: usize, over: f32| (back - (over - body.standing[point].y)).abs();
                            body.moved(if off(Body::HEAD, ON_HEAD) < off(Body::MIDDLE, 0.0) { Body::HEAD } else { Body::MIDDLE })
                        });
                        match facing.y > 0.3 {
                            true => (at + Vec2::new(away.x + seat.x, -1.0), back + BEHIND_THE_NECK - seat.y),
                            false => (at + Vec2::new(away.x + seat.x, 1.0 + away.y.max(0.0) * DEPTH_ON_BACK), back - seat.y),
                        }
                    }
                    // (Its middle between its hands as they go, a little before
                    // or behind it.)
                    (None, true) => match body {
                        Some(body) => {
                            let hands = body.hands() + Vec2::new(0.0, HELD_BELOW);
                            let before = (facing.y * BEFORE).max(hands.y);
                            (at + Vec2::new(hands.x, before) + facing.perp() * beside * APART, before - hands.y)
                        }
                        None => (at + facing * BEFORE + facing.perp() * beside * APART, IN_HANDS),
                    },
                    (None, false) => {
                        let head = body.map_or(Vec2::ZERO, |body| body.moved(Body::HEAD));
                        (at + Vec2::new(head.x, 1.0), OVER_HEAD - head.y)
                    }
                };
                (actor.position, actor.way.to, actor.way.left, actor.air.height) = (there, there, 0.0, high + lift);
                actor.attributes = (actor.attributes & !attribute::HIDDEN) | hidden;
                actor.looks.opacity = opacity;
            });
        }
    }
}

impl Actor {
    /// Who has it, if anyone does.
    pub fn carried(&self) -> bool {
        self.habits.carried_by.is_some()
    }
}
