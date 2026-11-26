//! Items and riders on DS. Sky's cutscenes show things as objects of its
//! object table, placed in the scene's layout and told by the script; an
//! item the film shows (held, given, thrown, eaten, set down, following a
//! path) is cast as one of `ITEM_SLOTS`, objects no scene of the story uses,
//! whose sprite `tools/ds/add_items.py` makes the item's icon (a picture of
//! the film's own: that picture). Sky's actors carry no one and hold
//! nothing: what someone carries or holds, or who rides him, is given each
//! of his moves, the same path and pace, offset to his hands or his back
//! and drawn in front (`Carried`). A path is its object's routine (`def n
//! for object`), its curve straight legs between its points.

use engine::{Beat, BodyBeat, Direction, FlowBeat, MotionBeat, NodeBeat, NodeRef, TrackMode, Who};

use super::{BESIDE, Writer};
use crate::notes::{Fate, Kind};
use crate::sky::{HIDDEN, ITEM_SLOTS, LAST_LOCK, direction, speed};

/// What an item's object shows: Sky's icon of an item, or a picture of the film's own.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Look {
    Item(u16),
    Picture(String),
}

/// A thing of the scene and the object showing it now.
#[derive(Clone, Debug)]
pub(super) struct Shown {
    pub(super) object: u16,
    /// Told something by the script since the scene began (its routine, if
    /// any, would end there).
    pub(super) ordered: bool,
    /// Its object has a routine (a path): another path is another object.
    pub(super) routine: bool,
    pub(super) height: f32,
    pub(super) hidden: bool,
}

/// Someone or something carried by another (held, ridden, lifted): given
/// each of its moves, that far from where it stands.
#[derive(Clone, Debug)]
pub(super) struct Carried {
    pub(super) who: Who,
    pub(super) on: Who,
    pub(super) dx: f32,
    pub(super) dy: f32,
}

/// A path of a thing's, its object's routine: started by the script
/// (`Unlock(gate)`), looping or not, waited for if the film waits (`done`).
#[derive(Clone, Debug)]
pub(super) struct Phase {
    pub(super) object: u16,
    pub(super) gate: u16,
    /// A new object for it: hidden until its path starts, put at its start.
    pub(super) fresh: bool,
    pub(super) start: Vec<String>,
    pub(super) legs: Vec<String>,
    pub(super) forever: bool,
    pub(super) done: Option<u16>,
    pub(super) frames: f32,
}

/// How high a thing is held (in the hands, over the head), how far in front
/// of the holder's feet it is drawn (a half tile: positions are in halves),
/// how high a thing handed over or thrown rises over the hands.
const HANDS: f32 = 10.0;
const OVER_HEAD: f32 = 30.0;
pub(super) const FRONT: f32 = 4.0;
const ARC: f32 = 16.0;
/// At once (a speed of the game's, pixels a frame).
const AT_ONCE: &str = "16";
/// The locks the items' paths may take, from the last down.
const PATH_LOCKS: u16 = 14;

/// Whether that one is a thing the export makes an object of (not a
/// place's own thing, which is the game's).
pub(super) fn item(who: Who) -> bool {
    matches!(who, Who::Item { .. } | Who::Drawn { .. })
}

/// The thing a beat is about, if it is about one the export makes an object of.
fn about(beat: &Beat) -> Option<Who> {
    let who = match beat {
        Beat::Body(body) => match body {
            BodyBeat::Carry { what, .. } | BodyBeat::Give { what, .. } => *what,
            BodyBeat::Put { who, .. }
            | BodyBeat::PutNear { who, .. }
            | BodyBeat::Turn { who, .. }
            | BodyBeat::Anim { who, .. }
            | BodyBeat::Pose { who, .. }
            | BodyBeat::Rise { who, .. }
            | BodyBeat::Attr { who, .. }
            | BodyBeat::Layer { who, .. }
            | BodyBeat::Blink { who, .. }
            | BodyBeat::Tremble { who, .. }
            | BodyBeat::Slow { who, .. }
            | BodyBeat::Freeze { who, .. }
            | BodyBeat::InFront { who, .. }
            | BodyBeat::Jump { who, .. }
            | BodyBeat::Hop { who, .. }
            | BodyBeat::Spin { who, .. }
            | BodyBeat::Opacity { who, .. }
            | BodyBeat::Scale { who, .. }
            | BodyBeat::Shadow { who, .. }
            | BodyBeat::Flip { who, .. }
            | BodyBeat::Sink { who, .. }
            | BodyBeat::Bounce { who, .. }
            | BodyBeat::ThingName { who, .. }
            | BodyBeat::Drawing { who, .. }
            | BodyBeat::Takeable { who, .. }
            | BodyBeat::Shine { who, .. }
            | BodyBeat::Throw { who, .. }
            | BodyBeat::Fall { who, .. }
            | BodyBeat::Float { who, .. }
            | BodyBeat::Colour { who, .. }
            | BodyBeat::Effect { who, .. }
            | BodyBeat::Fidget { who, .. } => *who,
            BodyBeat::Gone(who) | BodyBeat::Reset(who) | BodyBeat::Stop(who) => *who,
            _ => return None,
        },
        Beat::Motion(MotionBeat::Go { who, .. }) => *who,
        Beat::Node(NodeBeat::Path { node: NodeRef::Who(who), .. } | NodeBeat::Stop { node: NodeRef::Who(who), .. }) => *who,
        Beat::Flow(FlowBeat::WaitTrack(NodeRef::Who(who)) | FlowBeat::WaitFor(who) | FlowBeat::WaitMoves(who)) => *who,
        _ => return None,
    };
    item(who).then_some(who)
}

impl Writer<'_> {
    /// A beat about a thing the export makes an object of: played (or
    /// passed over: its name, its shadow...); false if it isn't about one.
    pub(super) fn thing_beat(&mut self, number: i32, beat: &Beat) -> bool {
        let Some(who) = about(beat) else { return false };
        self.item_lines.insert(number);
        match beat {
            Beat::Body(BodyBeat::Drawing { file, .. }) => {
                self.drawings.insert(who, file.clone());
            }
            Beat::Body(BodyBeat::Put { at, .. }) => {
                if let Some(point) = self.point(at) {
                    self.put_thing_at(number, who, point);
                }
            }
            Beat::Body(BodyBeat::PutNear { other, side, .. }) => {
                if let Some(&(x, y)) = self.at.get(other) {
                    let (dx, dy) = match side {
                        Direction::Left | Direction::UpLeft | Direction::DownLeft => (-BESIDE / 2.0, FRONT),
                        Direction::Right | Direction::UpRight | Direction::DownRight => (BESIDE / 2.0, FRONT),
                        Direction::Up => (0.0, -FRONT),
                        Direction::Down => (0.0, FRONT),
                    };
                    self.put_thing_at(number, who, (x + dx, y + dy));
                }
            }
            Beat::Body(BodyBeat::Carry { by, hands, .. }) => self.carry(number, who, *by, *hands),
            Beat::Body(BodyBeat::Give { from, to, frames, keeps, .. }) => self.hand_over(number, who, *from, *to, frames.0, *keeps),
            Beat::Body(BodyBeat::Throw { at, frames, .. }) => self.fly(number, who, *at, frames.0),
            Beat::Body(BodyBeat::Hop { at, frames, .. }) => {
                if let Some(to) = self.point(at) {
                    self.land(number, who, to, frames.0);
                }
            }
            Beat::Body(BodyBeat::Fall { frames, .. }) => self.lift_thing(number, who, 0.0, frames.0),
            Beat::Body(BodyBeat::Rise { height, frames, .. }) => self.lift_thing(number, who, *height, frames.0),
            Beat::Body(BodyBeat::Jump { height, frames, .. }) => {
                let now = self.things.get(&who).map_or(0.0, |shown| shown.height);
                self.lift_thing(number, who, now + height, frames.0 / 2.0);
                self.lift_thing(number, who, now, frames.0 / 2.0);
            }
            Beat::Body(BodyBeat::Gone(_)) => self.hide_thing(number, who),
            Beat::Body(BodyBeat::Opacity { to, .. }) if *to < 0.5 => self.hide_thing(number, who),
            Beat::Body(BodyBeat::Opacity { .. }) => {
                if self.object_of(number, who).is_some() {
                    self.reveal(who);
                }
            }
            Beat::Body(BodyBeat::Scale { .. }) => {
                if !self.notes.iter().any(|note| note.line == usize::try_from(number).ok() && note.kind == Kind::Item) {
                    self.note(number, Kind::Item, Fate::Changed, "an item growing or shrinking (bites)", "it keeps its size until it goes");
                }
            }
            Beat::Motion(MotionBeat::Go { at, frames, .. }) => {
                let (Some(to), Some(object)) = (self.point(at), self.object_of(number, who)) else { return true };
                self.carried.retain(|carried| carried.who != who);
                let from = self.at.get(&who).copied().unwrap_or(to);
                let pace = speed(distance(from, to), frames.0);
                let position = self.position(to);
                self.order(who, &format!("MovePositionMark<object OBJECT_N{object}>({pace}, {position});"));
                self.at.insert(who, to);
            }
            Beat::Node(NodeBeat::Path { points, frames, mode, relative, .. }) => {
                let base = if *relative { self.at.get(&who).copied().unwrap_or_default() } else { (0.0, 0.0) };
                let points: Vec<(f32, f32)> = points.iter().map(|point| (point.x + base.0, point.y + base.1)).collect();
                self.path(number, who, &points, frames.0, *mode);
            }
            Beat::Node(NodeBeat::Stop { .. }) => self.untrack(who),
            Beat::Flow(FlowBeat::WaitTrack(_)) => self.wait_track(who),
            Beat::Flow(FlowBeat::WaitFor(_) | FlowBeat::WaitMoves(_)) => {
                if let Some(shown) = self.things.get(&who).filter(|shown| shown.ordered && !shown.routine) {
                    let object = shown.object;
                    self.line(&format!("WaitExecuteObject(OBJECT_N{object});"));
                }
            }
            // (Its name, its shadow, whether it can be taken, how it is drawn...: nothing on DS.)
            _ => {}
        }
        true
    }

    /// The object showing that thing now, cast the first time (hidden until
    /// put, if the scene has begun).
    pub(super) fn object_of(&mut self, number: i32, who: Who) -> Option<u16> {
        if let Some(shown) = self.things.get(&who) {
            return Some(shown.object);
        }
        let look = match who {
            Who::Item { item, .. } => Look::Item(item),
            Who::Drawn { .. } => match self.drawings.get(&who) {
                Some(file) => Look::Picture(file.clone()),
                None => {
                    self.problem(number, Kind::Item, Fate::LeftOut, "a picture of the film's own whose file isn't known", "nothing");
                    return None;
                }
            },
            _ => return None,
        };
        let at = self.at.get(&who).copied().unwrap_or(self.camera);
        let object = self.new_object(number, look, at)?;
        let hidden = self.started;
        if hidden {
            self.hidden_objects.push(object);
        }
        self.things.insert(who, Shown { object, ordered: false, routine: false, height: 0.0, hidden });
        Some(object)
    }

    /// One more object of that look in this scene, placed there in its
    /// layout: the film's slot for it (the n-th of that look in a scene is
    /// the same slot in every scene).
    fn new_object(&mut self, number: i32, look: Look, at: (f32, f32)) -> Option<u16> {
        let count = self.look_count.entry(look.clone()).or_default();
        let key = (look, *count);
        *count += 1;
        let slot = match self.slots.get(&key) {
            Some(slot) => *slot,
            None => {
                let Some(free) = ITEM_SLOTS.iter().copied().find(|slot| !self.slots.values().any(|used| used == slot)) else {
                    self.problem(
                        number,
                        Kind::Item,
                        Fate::LeftOut,
                        format!("more than {} items (and paths) in the film", ITEM_SLOTS.len()),
                        "this one not shown",
                    );
                    return None;
                };
                self.slots.insert(key, free);
                free
            }
        };
        self.objects.insert(slot, at);
        Some(slot)
    }

    /// A line telling a thing's object something (its routine, if any, ends).
    fn order(&mut self, who: Who, text: &str) {
        if let Some(shown) = self.things.get_mut(&who) {
            shown.ordered = true;
        }
        self.line(text);
    }

    /// Shown, if it was hidden.
    fn reveal(&mut self, who: Who) {
        let Some(shown) = self.things.get_mut(&who).filter(|shown| shown.hidden) else { return };
        shown.hidden = false;
        let object = shown.object;
        self.hidden_objects.retain(|one| *one != object);
        if self.started {
            self.order(who, &format!("ResetOutputAttribute<object OBJECT_N{object}>({HIDDEN});"));
        }
    }

    fn hide_thing(&mut self, number: i32, who: Who) {
        let Some(object) = self.object_of(number, who) else { return };
        self.carried.retain(|carried| carried.who != who);
        if let Some(shown) = self.things.get_mut(&who) {
            if shown.hidden {
                return;
            }
            shown.hidden = true;
        }
        self.order(who, &format!("SetOutputAttribute<object OBJECT_N{object}>({HIDDEN});"));
    }

    /// Put there, on the ground, and shown (before the scene: where its layout has it).
    fn put_thing_at(&mut self, number: i32, who: Who, point: (f32, f32)) {
        let Some(object) = self.object_of(number, who) else { return };
        self.carried.retain(|carried| carried.who != who);
        self.at.insert(who, point);
        if !self.started && !self.things.get(&who).is_some_and(|shown| shown.ordered) {
            self.objects.insert(object, point);
            if let Some(shown) = self.things.get_mut(&who) {
                shown.hidden = false;
            }
            self.hidden_objects.retain(|one| *one != object);
        } else {
            let position = self.position(point);
            self.order(who, &format!("SetPositionMark<object OBJECT_N{object}>({position});"));
        }
        self.lift_thing(number, who, 0.0, 0.0);
        self.reveal(who);
    }

    /// Up or down to that height, in so many frames (none: at once).
    fn lift_thing(&mut self, number: i32, who: Who, height: f32, frames: f32) {
        let Some(object) = self.object_of(number, who) else { return };
        let now = self.things.get(&who).map_or(0.0, |shown| shown.height);
        if (now - height).abs() < 1.0 {
            return;
        }
        let pace = if frames < 1.0 { AT_ONCE.to_string() } else { speed((now - height).abs(), frames) };
        if let Some(shown) = self.things.get_mut(&who) {
            shown.height = height;
        }
        self.order(who, &format!("MoveHeight<object OBJECT_N{object}>({pace}, {});", height.round() as i32));
    }

    /// Carried by someone (in the hands, or over the head), wherever he
    /// goes; none: let go, where it is.
    fn carry(&mut self, number: i32, who: Who, by: Option<Who>, hands: bool) {
        self.carried.retain(|carried| carried.who != who);
        let Some(by) = by else { return };
        let Some(object) = self.object_of(number, who) else { return };
        let Some(&(x, y)) = self.at.get(&by) else { return };
        let back = self.facing.get(&by).is_some_and(|way| matches!(way, Direction::Up | Direction::UpLeft | Direction::UpRight));
        let dy = if back { -FRONT } else { FRONT };
        let point = (x, y + dy);
        self.at.insert(who, point);
        if !self.started && !self.things.get(&who).is_some_and(|shown| shown.ordered) {
            self.objects.insert(object, point);
        } else {
            let position = self.position(point);
            self.order(who, &format!("SetPositionMark<object OBJECT_N{object}>({position});"));
        }
        self.lift_thing(number, who, if hands { HANDS } else { OVER_HEAD }, 0.0);
        self.carried.push(Carried { who, on: by, dx: 0.0, dy });
    }

    /// Handed from someone to someone else: up over the giver's hands,
    /// across, down into the other's (an arc as three steps); kept there, or gone.
    fn hand_over(&mut self, number: i32, who: Who, from: Who, to: Who, frames: f32, keeps: bool) {
        let Some(object) = self.object_of(number, who) else { return };
        self.carried.retain(|carried| carried.who != who);
        let (Some(&giver), Some(&taker)) = (self.at.get(&from), self.at.get(&to)) else { return };
        if self.things.get(&who).is_some_and(|shown| shown.hidden) {
            let position = self.position((giver.0, giver.1 + FRONT));
            self.order(who, &format!("SetPositionMark<object OBJECT_N{object}>({position});"));
            self.at.insert(who, (giver.0, giver.1 + FRONT));
            self.lift_thing(number, who, HANDS, 0.0);
            self.reveal(who);
        }
        self.arc(number, who, (taker.0, taker.1 + FRONT), frames);
        if keeps {
            self.carried.push(Carried { who, on: to, dx: 0.0, dy: FRONT });
        } else {
            self.hide_thing(number, who);
        }
    }

    /// Thrown at someone: up, across to before him, at the height of his hands.
    fn fly(&mut self, number: i32, who: Who, at: Who, frames: f32) {
        self.carried.retain(|carried| carried.who != who);
        let Some(&(x, y)) = self.at.get(&at) else { return };
        self.arc(number, who, (x, y + FRONT), frames);
    }

    /// Up over the hands, across in so many frames, down to the hands
    /// (each step waited for: an order ends the one before).
    fn arc(&mut self, number: i32, who: Who, to: (f32, f32), frames: f32) {
        let Some(object) = self.object_of(number, who) else { return };
        let from = self.at.get(&who).copied().unwrap_or(to);
        self.lift_thing(number, who, HANDS + ARC, 6.0);
        self.line(&format!("WaitExecuteObject(OBJECT_N{object});"));
        let position = self.position(to);
        let pace = speed(distance(from, to), (frames - 12.0).max(6.0));
        self.order(who, &format!("MovePositionMark<object OBJECT_N{object}>({pace}, {position});"));
        self.line(&format!("WaitExecuteObject(OBJECT_N{object});"));
        self.at.insert(who, to);
        self.lift_thing(number, who, HANDS, 6.0);
        if !self.notes.iter().any(|note| note.line == usize::try_from(number).ok() && note.kind == Kind::Item) {
            self.note(number, Kind::Item, Fate::Changed, "an item's arc through the air", "up, across, down: three steps, the script waiting for them");
        }
    }

    /// To a place in so many frames, then down to the ground.
    fn land(&mut self, number: i32, who: Who, to: (f32, f32), frames: f32) {
        let Some(object) = self.object_of(number, who) else { return };
        self.carried.retain(|carried| carried.who != who);
        let from = self.at.get(&who).copied().unwrap_or(to);
        let position = self.position(to);
        let pace = speed(distance(from, to).max(1.0), frames * 0.6);
        self.reveal(who);
        self.order(who, &format!("MovePositionMark<object OBJECT_N{object}>({pace}, {position});"));
        self.line(&format!("WaitExecuteObject(OBJECT_N{object});"));
        self.at.insert(who, to);
        self.lift_thing(number, who, 0.0, (frames * 0.4).max(4.0));
    }

    /// A path followed, as its object's routine: straight legs between its
    /// points at one pace (its curve and easing lost), looping if it loops.
    /// An object told something since the scene began, or that had a path
    /// already, can't start a routine: a new object of the same look takes
    /// its place there (the old one hidden).
    fn path(&mut self, number: i32, who: Who, points: &[(f32, f32)], frames: f32, mode: TrackMode) {
        let Some(&first) = points.first() else { return };
        let Some(old) = self.object_of(number, who) else { return };
        self.carried.retain(|carried| carried.who != who);
        let from = self.at.get(&who).copied().unwrap_or(first);
        if self.path_locks >= PATH_LOCKS {
            self.note(number, Kind::Timing, Fate::Changed, "too many paths in the scene for Sky's locks", "to its last point at once");
            let last = *points.last().unwrap_or(&first);
            self.put_thing_at(number, who, last);
            return;
        }
        let (object, fresh) = match self.things.get(&who) {
            Some(shown) if !shown.ordered && !shown.routine && !shown.hidden && self.started => (old, false),
            Some(shown) if !shown.routine && !self.started => (old, false),
            _ => {
                let look = match who {
                    Who::Item { item, .. } => Look::Item(item),
                    _ => match self.drawings.get(&who) {
                        Some(file) => Look::Picture(file.clone()),
                        None => return,
                    },
                };
                let Some(new) = self.new_object(number, look, first) else { return };
                let shown = self.things.get(&who).is_some_and(|shown| !shown.hidden);
                if shown {
                    self.order(who, &format!("SetOutputAttribute<object OBJECT_N{old}>({HIDDEN});"));
                }
                self.hidden_objects.retain(|one| *one != new);
                (new, true)
            }
        };
        let gate = LAST_LOCK - self.path_locks;
        self.path_locks += 1;
        // The legs: from where it is to the first point (an object of its
        // own starts there), between the points, back to the first if it loops.
        let mut stops: Vec<(f32, f32)> = Vec::new();
        let mut start = Vec::new();
        if fresh {
            let position = self.position(first);
            start.push(format!("SetPositionMark({position});"));
            start.push(format!("ResetOutputAttribute({HIDDEN});"));
        } else if distance(from, first) >= 2.0 {
            stops.push(from);
        }
        stops.extend_from_slice(points);
        let mut route: Vec<(f32, f32)> = stops.clone();
        match mode {
            TrackMode::Loop if distance(*points.last().unwrap_or(&first), first) >= 2.0 => route.push(first),
            TrackMode::PingPong => route.extend(points.iter().rev().skip(1).copied()),
            _ => {}
        }
        let length: f32 = route.windows(2).map(|leg| distance(leg[0], leg[1])).sum();
        let pace = speed(length.max(1.0), frames);
        let mut legs = Vec::new();
        for to in route.iter().skip(1) {
            let position = self.position(*to);
            legs.push(format!("MovePositionMark({pace}, {position});"));
        }
        let forever = mode != TrackMode::Once;
        self.phases.push(Phase { object, gate, fresh, start, legs, forever, done: None, frames });
        self.line(&format!("WaitLockObject({gate}, OBJECT_N{object});"));
        self.line(&format!("Unlock({gate});"));
        let height = self.things.get(&who).map_or(0.0, |shown| shown.height);
        self.things.insert(who, Shown { object, ordered: false, routine: true, height: if fresh { 0.0 } else { height }, hidden: false });
        self.at.insert(who, if forever { first } else { *points.last().unwrap_or(&first) });
        if !self.notes.iter().any(|note| note.line == usize::try_from(number).ok() && note.kind == Kind::Item) {
            self.note(number, Kind::Item, Fate::Changed, "a path's curve and easing", "straight legs between its points, at one pace (its object's routine)");
        }
    }

    /// No longer on its path: where it is (an order ends its routine).
    fn untrack(&mut self, who: Who) {
        let Some(shown) = self.things.get(&who).filter(|shown| shown.routine && !shown.ordered) else { return };
        let (object, height) = (shown.object, shown.height);
        self.order(who, &format!("MoveHeight<object OBJECT_N{object}>({AT_ONCE}, {});", height.round() as i32));
    }

    /// The script waits for a thing's path to end.
    fn wait_track(&mut self, who: Who) {
        let Some(object) = self.things.get(&who).filter(|shown| shown.routine).map(|shown| shown.object) else { return };
        let Some(at) = self.phases.iter().rposition(|phase| phase.object == object && !phase.forever) else { return };
        let done = match self.phases[at].done {
            Some(done) => done,
            None if self.path_locks < PATH_LOCKS => {
                let done = LAST_LOCK - self.path_locks;
                self.path_locks += 1;
                self.phases[at].done = Some(done);
                done
            }
            None => {
                let frames = self.phases[at].frames.round() as i32;
                self.line(&format!("Wait({frames});"));
                return;
            }
        };
        self.line(&format!("WaitLockObject({done}, OBJECT_N{object});"));
        self.line(&format!("Unlock({done});"));
    }

    /// What someone carries, or who rides him, given his move there at that
    /// pace (a rider slides, turned as his mount; a thing goes).
    pub(super) fn carry_along(&mut self, on: Who, to: (f32, f32), pace: &str) {
        let riding: Vec<Carried> = self.carried.iter().filter(|carried| carried.on == on).cloned().collect();
        for carried in riding {
            let point = (to.0 + carried.dx, to.1 + carried.dy);
            let position = self.position(point);
            if item(carried.who) {
                let Some(object) = self.things.get(&carried.who).map(|shown| shown.object) else { continue };
                self.order(carried.who, &format!("MovePositionMark<object OBJECT_N{object}>({pace}, {position});"));
            } else if let Some(actor) = self.actors.get(&carried.who).map(|actor| format!("ACTOR_{}", actor.name)) {
                let way = self.facing.get(&on).copied().unwrap_or(Direction::Down);
                self.facing.insert(carried.who, way);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(way)));
                self.line(&format!("SlidePositionMark<actor {actor}>({pace}, {position});"));
                self.moving(&actor);
            }
            self.at.insert(carried.who, point);
            self.carry_along(carried.who, point, pace);
        }
    }

    /// What someone carries, or who rides him, put where he is put.
    pub(super) fn set_along(&mut self, on: Who, to: (f32, f32)) {
        let riding: Vec<Carried> = self.carried.iter().filter(|carried| carried.on == on).cloned().collect();
        for carried in riding {
            let point = (to.0 + carried.dx, to.1 + carried.dy);
            let position = self.position(point);
            if item(carried.who) {
                let Some(object) = self.things.get(&carried.who).map(|shown| shown.object) else { continue };
                self.order(carried.who, &format!("SetPositionMark<object OBJECT_N{object}>({position});"));
            } else if let Some(actor) = self.actors.get(&carried.who).map(|actor| format!("ACTOR_{}", actor.name)) {
                self.line(&format!("SetPositionMark<actor {actor}>({position});"));
            }
            self.at.insert(carried.who, point);
            self.set_along(carried.who, point);
        }
    }

    /// The script waits for what someone carries and who rides him, as it
    /// waits for him.
    pub(super) fn wait_along(&mut self, actor: &str) {
        let Some(on) = self.actors.iter().find(|(_, one)| format!("ACTOR_{}", one.name) == actor).map(|(who, _)| *who) else { return };
        let riding: Vec<Who> = self.carried.iter().filter(|carried| carried.on == on).map(|carried| carried.who).collect();
        for who in riding {
            if item(who) {
                if let Some(object) = self.things.get(&who).filter(|shown| shown.ordered).map(|shown| shown.object) {
                    self.line(&format!("WaitExecuteObject(OBJECT_N{object});"));
                }
            } else if let Some(rider) = self.actors.get(&who).map(|one| format!("ACTOR_{}", one.name)) {
                self.busy.retain(|one| *one != rider);
                self.line(&format!("WaitExecuteLives({rider});"));
            }
        }
    }

    /// On someone's back (or head), drawn in front, wherever he goes: put
    /// there, that high; none: off, where he is.
    pub(super) fn ride(&mut self, who: Who, on: Option<Who>) {
        self.carried.retain(|carried| carried.who != who);
        let Some(mount) = on else { return };
        let Some(&(x, y)) = self.at.get(&mount) else { return };
        let Some(actor) = self.actors.get(&who).map(|one| format!("ACTOR_{}", one.name)) else { return };
        let point = (x, y + FRONT);
        self.at.insert(who, point);
        let position = self.position(point);
        self.line(&format!("SetPositionMark<actor {actor}>({position});"));
        self.carried.push(Carried { who, on: mount, dx: 0.0, dy: FRONT });
    }

    /// The items' paths as their objects' routines (`def n for object`),
    /// numbered from `first`.
    pub(super) fn object_routines(&self, first: usize) -> String {
        let mut defs = String::new();
        for (at, phase) in self.phases.iter().enumerate() {
            defs.push_str(&format!("\ndef {} for object OBJECT_N{} {{\n", first + at, phase.object));
            if phase.fresh {
                defs.push_str(&format!("    SetOutputAttribute({HIDDEN});\n"));
            }
            defs.push_str(&format!("    Lock({});\n", phase.gate));
            for line in &phase.start {
                defs.push_str(&format!("    {line}\n"));
            }
            if phase.forever {
                defs.push_str("    forever {\n");
                for line in &phase.legs {
                    defs.push_str(&format!("        {line}\n"));
                }
                defs.push_str("    }\n");
            } else {
                for line in &phase.legs {
                    defs.push_str(&format!("    {line}\n"));
                }
                if let Some(done) = phase.done {
                    defs.push_str(&format!("    Lock({done});\n"));
                }
                defs.push_str("    hold;\n");
            }
            defs.push_str("}\n");
        }
        defs
    }

    /// The film's items, a line each for `patch_rom.py` (`// ds-export item
    /// OBJECT_N<slot> item <id>`); its pictures, (slot, file), for the export
    /// to write beside it.
    pub(crate) fn item_lines_for_patch(&self) -> (String, Vec<(u16, String)>) {
        let mut lines = String::new();
        let mut pictures = Vec::new();
        for ((look, _), slot) in &self.slots {
            match look {
                Look::Item(item) => lines.push_str(&format!("// ds-export item OBJECT_N{slot} item {item}\n")),
                Look::Picture(file) => pictures.push((*slot, file.clone())),
            }
        }
        (lines, pictures)
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}
