//! Tracks: a property of any node going through keys in time ([`Track`]),
//! or a node going along a path at an even speed. The stage holds them
//! (`Stage::tracks`) and plays them frame by frame (`Stage::pass_tracks`),
//! each in its node's own time (a body slowed down, its tracks are), and
//! writes what they give by the registry's names (`Stage::set`): whatever
//! the registry says can animate, a track animates. A property joins them
//! by being in the registry, animatable, and kept by `Stage::get` and
//! `Stage::set`; nothing here knows it.
//!
//! How each kind of value goes between two keys: numbers and points
//! evenly (by the key's ease), colours part by part (with their alpha),
//! an angle the short way round (unless the track is `exact` or the key
//! relative), on/off, choices and none at once as the key is reached.

use super::*;
use crate::beat::{Key, KeyTime, NodeRef, PathShape, TrackMode};
use crate::props::{Given, PropId, Value};

/// A key of a track as it plays: so many frames from its start, its value
/// worked out as the track started, how it comes to it, and whether an
/// angle goes all the way to it (not the short way round).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackKey {
    pub at: f32,
    pub value: Value,
    pub ease: Ease,
    pub whole_way: bool,
}

/// A way along points, as it plays: the points of its curve close
/// together, and how far along it each is.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackWay {
    pub points: Vec<Vec2>,
    pub along: Vec<f32>,
    pub frames: f32,
    pub ease: Ease,
    pub orient: bool,
}

/// What a track goes through: keys of a property, or a way for a node's
/// position.
#[derive(Clone, Debug, PartialEq)]
pub enum Through {
    Keys(Vec<TrackKey>),
    Way(TrackWay),
}

/// A property of a node going through its course, so many frames gone.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub node: NodeRef,
    pub prop: PropId,
    pub course: Through,
    pub mode: TrackMode,
    pub gone: f32,
}

impl Track {
    /// How many frames a round of it lasts.
    pub fn length(&self) -> f32 {
        match &self.course {
            Through::Keys(keys) => keys.last().map_or(0.0, |key| key.at),
            Through::Way(way) => way.frames,
        }
    }

    /// Whether it ends (one that loops never does).
    pub fn ends(&self) -> bool {
        self.mode == TrackMode::Once
    }

    /// How many keys (a way: its points).
    pub fn keys(&self) -> usize {
        match &self.course {
            Through::Keys(keys) => keys.len(),
            Through::Way(way) => way.points.len(),
        }
    }

    /// Where in a round it is, so many frames gone: looped, or back and
    /// forth; and whether it goes back now.
    fn in_round(&self, gone: f32) -> (f32, bool) {
        let length = self.length();
        if length <= 0.0 {
            return (length, false);
        }
        match self.mode {
            TrackMode::Once => (gone.min(length), false),
            TrackMode::Loop => (gone.rem_euclid(length), false),
            TrackMode::PingPong => {
                let twice = gone.rem_euclid(2.0 * length);
                if twice <= length { (twice, false) } else { (2.0 * length - twice, true) }
            }
        }
    }

    /// What it gives so many frames gone: the value, and for a way turned
    /// the way it goes, which way that is.
    pub fn at(&self, gone: f32) -> (Value, Option<Vec2>) {
        let (t, back) = self.in_round(gone);
        match &self.course {
            Through::Keys(keys) => (keyed(keys, t, self.prop), None),
            Through::Way(way) => {
                let done = if way.frames <= 0.0 { 1.0 } else { t / way.frames };
                let far = way.ease.along(done) * way.along.last().copied().unwrap_or(0.0);
                let at = way.point(far);
                let ahead = way.point(far + if back { -1.0 } else { 1.0 });
                let behind = way.point(far + if back { 1.0 } else { -1.0 });
                let heading = (ahead - behind).try_normalize();
                (Value::Point(at), heading.filter(|_| way.orient))
            }
        }
    }

    /// Whether it is over, so many frames gone.
    fn over(&self, gone: f32) -> bool {
        self.ends() && gone >= self.length()
    }
}

/// The value of keys so many frames in: between the two it is between,
/// by the ease of the later.
fn keyed(keys: &[TrackKey], t: f32, prop: PropId) -> Value {
    let Some(first) = keys.first() else {
        return Value::Nothing;
    };
    if t <= first.at {
        return first.value;
    }
    for pair in keys.windows(2) {
        let (from, to) = (&pair[0], &pair[1]);
        if t <= to.at {
            let span = to.at - from.at;
            let done = if span <= 0.0 { 1.0 } else { (t - from.at) / span };
            return between(from.value, to.value, to.ease.along(done), done >= 1.0, prop == PropId::Angle && !to.whole_way);
        }
    }
    keys.last().map_or(Value::Nothing, |key| key.value)
}

/// A value that far (0 to 1, past them for back and elastic) from one to
/// another: numbers and points evenly, colours part by part, an angle the
/// short way round if `short`; what does not go by degrees (on/off, a
/// choice, none), the first until the end is reached.
pub fn between(from: Value, to: Value, part: f32, reached: bool, short: bool) -> Value {
    match (from, to) {
        (Value::Number(from), Value::Number(to)) if short => {
            if reached {
                return Value::Number(to);
            }
            let turn = (to - from + 180.0).rem_euclid(360.0) - 180.0;
            Value::Number(from + turn * part)
        }
        (Value::Number(from), Value::Number(to)) => Value::Number(from + (to - from) * part),
        (Value::Point(from), Value::Point(to)) => Value::Point(from + (to - from) * part),
        (Value::Colour(from), Value::Colour(to)) => {
            Value::Colour(std::array::from_fn(|at| (f32::from(from[at]) + (f32::from(to[at]) - f32::from(from[at])) * part).round().clamp(0.0, 255.0) as u8))
        }
        (_, to) if reached => to,
        (from, _) => from,
    }
}

/// How many points each piece of a curve is cut into.
const CUTS: usize = 16;

impl TrackWay {
    /// A way through points, of that shape: cut into short straight
    /// pieces, each so far along.
    pub fn new(points: &[Vec2], shape: PathShape, frames: f32, ease: Ease, orient: bool) -> TrackWay {
        let curve: Vec<Vec2> = match (shape, points.len()) {
            (_, 0 | 1) | (PathShape::Lines, _) => points.to_vec(),
            (PathShape::CatmullRom, _) => catmull_rom(points),
            (PathShape::Bezier, _) => {
                (0..=CUTS * (points.len() - 1)).map(|cut| de_casteljau(points, cut as f32 / (CUTS * (points.len() - 1)) as f32)).collect()
            }
        };
        let mut along = Vec::with_capacity(curve.len());
        let mut far = 0.0;
        for (at, point) in curve.iter().enumerate() {
            if at > 0 {
                far += point.distance(curve[at - 1]);
            }
            along.push(far);
        }
        TrackWay { points: curve, along, frames, ease, orient }
    }

    /// The point so far along it (before it: its start; past it: its end).
    pub fn point(&self, far: f32) -> Vec2 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return Vec2::ZERO;
        };
        if far <= 0.0 {
            return *first;
        }
        let piece = self.along.partition_point(|along| *along < far);
        if piece >= self.points.len() {
            return *last;
        }
        if piece == 0 {
            return *first;
        }
        let (from, to) = (self.along[piece - 1], self.along[piece]);
        let part = if to > from { (far - from) / (to - from) } else { 1.0 };
        self.points[piece - 1].lerp(self.points[piece], part)
    }
}

/// A Catmull-Rom curve through every point (its ends doubled), cut.
fn catmull_rom(points: &[Vec2]) -> Vec<Vec2> {
    let at = |index: isize| points[index.clamp(0, points.len() as isize - 1) as usize];
    let mut curve = vec![points[0]];
    for piece in 0..points.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(piece - 1), at(piece), at(piece + 1), at(piece + 2));
        for cut in 1..=CUTS {
            let t = cut as f32 / CUTS as f32;
            let (t2, t3) = (t * t, t * t * t);
            curve.push(0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3));
        }
    }
    curve
}

/// The point of the bezier curve whose controls the points are, `t` along.
fn de_casteljau(points: &[Vec2], t: f32) -> Vec2 {
    let mut level = points.to_vec();
    while level.len() > 1 {
        level = level.windows(2).map(|pair| pair[0].lerp(pair[1], t)).collect();
    }
    level[0]
}

impl Stage {
    /// Starts a track of a node's property: its keys worked out from what
    /// the property is now (a first key later than 0: from it, as it is);
    /// any track of the same property goes. False if the node is not
    /// there, has no such property or the property does not animate.
    pub fn start_track(&mut self, node: &NodeRef, prop: PropId, keys: &[Key], mode: TrackMode, relative: bool, exact: bool) -> bool {
        let Some(now) = self.get(node, prop) else {
            return false;
        };
        if !prop.prop().animatable || keys.is_empty() {
            return false;
        }
        let mut written: Vec<(f32, &Key)> = keys
            .iter()
            .map(|key| {
                let at = match &key.at {
                    KeyTime::After(frames) => frames.0,
                    // (Not timed by the director: at once.)
                    _ => 0.0,
                };
                (at.max(0.0), key)
            })
            .collect();
        written.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut worked = Vec::new();
        if written[0].0 > 0.0 {
            worked.push(TrackKey { at: 0.0, value: now, ease: Ease::Linear, whole_way: false });
        }
        let registered = prop.prop();
        for (at, key) in written {
            let before = worked.last().map_or(now, |key: &TrackKey| key.value);
            let (value, whole_way) = match &key.value {
                Given::Is(value) if relative => (now.plus(*value), true),
                Given::Is(value) => (Some(*value), exact),
                Given::Plus(by) => (before.plus(*by), true),
                Given::Times(by) => (before.times(*by), true),
                Given::At { node: other, by } => (self.get(other, PropId::Position).and_then(|at| at.point()).map(|at| Value::Point(at + *by)), exact),
                Given::Default => (Some(registered.default), exact),
            };
            // (A value that cannot be worked out: as the key before.)
            worked.push(TrackKey { at, value: value.unwrap_or(before), ease: key.ease, whole_way });
        }
        let track = Track { node: node.clone(), prop, course: Through::Keys(worked), mode, gone: 0.0 };
        self.tracks.retain(|other| !(other.node == *node && other.prop == prop));
        let (first, _) = track.at(0.0);
        self.tracks.push(track);
        _ = self.set(node, prop, &Given::Is(first));
        true
    }

    /// Starts a node going along points (from where it is, if `relative`):
    /// a track of its position. False if the node is not there.
    pub fn start_path(
        &mut self,
        node: &NodeRef,
        points: &[Vec2],
        shape: PathShape,
        frames: f32,
        ease: Ease,
        mode: TrackMode,
        orient: bool,
        relative: bool,
    ) -> bool {
        let Some(Value::Point(now)) = self.get(node, PropId::Position) else {
            return false;
        };
        let shift = match (relative, points.first()) {
            (true, Some(first)) => now - *first,
            _ => Vec2::ZERO,
        };
        let points: Vec<Vec2> = points.iter().map(|at| *at + shift).collect();
        let way = TrackWay::new(&points, shape, frames.max(0.0), ease, orient);
        self.tracks.retain(|other| !(other.node == *node && matches!(other.prop, PropId::Position | PropId::X | PropId::Y)));
        self.tracks.push(Track { node: node.clone(), prop: PropId::Position, course: Through::Way(way), mode, gone: 0.0 });
        self.pass_track(self.tracks.len() - 1, 0.0);
        true
    }

    /// Stops a node's tracks (of that property only, if said): it stays as
    /// it is.
    pub fn stop_tracks(&mut self, node: &NodeRef, prop: Option<PropId>) {
        self.tracks.retain(|track| !(track.node == *node && prop.is_none_or(|prop| track.prop == prop)));
    }

    /// Whether a node has a track still to end (one that loops is not).
    pub fn tracked(&self, node: &NodeRef) -> bool {
        self.tracks.iter().any(|track| track.node == *node && track.ends())
    }

    /// The tracks: each goes so many frames on, in its node's own time,
    /// and sets what it gives; those over, and those whose node is gone,
    /// go.
    pub(super) fn pass_tracks(&mut self, frames: f32) {
        let mut at = 0;
        while at < self.tracks.len() {
            let time = match &self.tracks[at].node {
                NodeRef::Who(who) | NodeRef::Emitter(who) => self.cast.get(who).map_or(1.0, |actor| actor.looks.time),
                _ => 1.0,
            };
            if self.pass_track(at, frames * time) {
                at += 1;
            } else {
                self.tracks.remove(at);
            }
        }
    }

    /// A track so many frames on, what it gives set; whether it goes on.
    fn pass_track(&mut self, at: usize, frames: f32) -> bool {
        let track = &mut self.tracks[at];
        track.gone += frames;
        let (gone, node, prop) = (track.gone, track.node.clone(), track.prop);
        let over = track.over(gone);
        let (value, heading) = track.at(gone);
        if self.set(&node, prop, &Given::Is(value)) == Err(SetError::NoNode) {
            return false;
        }
        if let Some(heading) = heading {
            self.face_along(&node, heading);
        }
        !over
    }

    /// A node turned the way it goes: a body faces it, anything else is
    /// turned to it (0: to the right, clockwise).
    fn face_along(&mut self, node: &NodeRef, heading: Vec2) {
        match node {
            NodeRef::Who(who) => {
                if let Some(actor) = self.cast.get_mut(who) {
                    actor.direction = towards(Vec2::ZERO, heading).unwrap_or(actor.direction);
                }
            }
            _ => _ = self.set(node, PropId::Angle, &Given::Is(Value::Number(heading.y.atan2(heading.x).to_degrees()))),
        }
    }

    /// The tracks that end, at their end at once; those that loop go on.
    pub(super) fn settle_tracks(&mut self) {
        let mut at = 0;
        while at < self.tracks.len() {
            if self.tracks[at].ends() {
                let length = self.tracks[at].length();
                self.pass_track(at, length - self.tracks[at].gone + 1.0);
                self.tracks.remove(at);
            } else {
                at += 1;
            }
        }
    }

    /// Plays a track's beat.
    pub(super) fn direct_track(&mut self, beat: NodeBeat) {
        match beat {
            NodeBeat::Animate { node, prop, keys, mode, relative, exact } => _ = self.start_track(&node, prop, &keys, mode, relative, exact),
            NodeBeat::Path { node, points, shape, frames, ease, mode, orient, relative } => {
                _ = self.start_path(&node, &points, shape, frames.0, ease, mode, orient, relative)
            }
            NodeBeat::Stop { node, prop } => self.stop_tracks(&node, prop),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{EaseFamily, Frames};
    use crate::beat::{Key, KeyTime};
    use crate::who::Who;

    fn pika() -> NodeRef {
        NodeRef::Who(Who::Monster(25))
    }

    fn stage() -> Stage {
        let mut stage = Stage::default();
        stage.put(Who::Monster(25), Vec2::new(100.0, 100.0), 0);
        stage
    }

    fn key(at: f32, value: Given<NodeRef>, ease: Ease) -> Key {
        Key { at: KeyTime::After(Frames(at)), value, ease }
    }

    fn number(value: f32) -> Given<NodeRef> {
        Given::Is(Value::Number(value))
    }

    fn get(stage: &Stage, prop: PropId) -> Value {
        stage.get(&pika(), prop).unwrap()
    }

    fn close(value: Value, to: f32) -> bool {
        value.number().is_some_and(|number| (number - to).abs() < 1e-3)
    }

    #[test]
    fn a_track_plays_its_keys_and_ends() {
        let mut stage = stage();
        assert!(stage.start_track(
            &pika(),
            PropId::Opacity,
            &[key(0.0, number(0.0), Ease::Linear), key(60.0, number(1.0), Ease::Linear)],
            TrackMode::Once,
            false,
            false
        ));
        assert!(close(get(&stage, PropId::Opacity), 0.0), "it starts at its first key");
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Opacity), 0.5));
        assert!(stage.tracked(&pika()) && stage.busy());
        stage.pass_tracks(40.0);
        assert!(close(get(&stage, PropId::Opacity), 1.0) && stage.tracks.is_empty() && !stage.tracked(&pika()));
    }

    #[test]
    fn a_first_key_later_starts_from_what_it_is() {
        let mut stage = stage();
        stage.start_track(&pika(), PropId::X, &[key(60.0, number(160.0), Ease::Linear)], TrackMode::Once, false, false);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::X), 130.0), "{:?}", get(&stage, PropId::X));
    }

    #[test]
    fn eases_shape_a_track() {
        let mut stage = stage();
        stage.start_track(
            &pika(),
            PropId::X,
            &[key(0.0, number(0.0), Ease::Linear), key(60.0, number(100.0), Ease::In(EaseFamily::Quad))],
            TrackMode::Once,
            false,
            false,
        );
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::X), 25.0));
    }

    #[test]
    fn relative_keys_add_to_the_one_before() {
        let mut stage = stage();
        let keys = [key(30.0, Given::Plus(Value::Number(10.0)), Ease::Linear), key(60.0, Given::Plus(Value::Number(10.0)), Ease::Linear)];
        stage.start_track(&pika(), PropId::X, &keys, TrackMode::Once, false, false);
        stage.pass_tracks(60.0);
        assert!(close(get(&stage, PropId::X), 120.0));
        // (All of them from where it started.)
        stage.start_track(&pika(), PropId::Y, &[key(30.0, number(5.0), Ease::Linear), key(60.0, number(-5.0), Ease::Linear)], TrackMode::Once, true, false);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Y), 105.0));
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Y), 95.0));
    }

    #[test]
    fn loops_and_ping_pong_go_on() {
        let mut stage = stage();
        let keys = [key(0.0, number(0.0), Ease::Linear), key(60.0, number(60.0), Ease::Linear)];
        stage.start_track(&pika(), PropId::X, &keys, TrackMode::Loop, false, false);
        stage.pass_tracks(90.0);
        assert!(close(get(&stage, PropId::X), 30.0));
        assert!(!stage.tracked(&pika()) && !stage.busy(), "a loop is not waited for");
        stage.start_track(&pika(), PropId::X, &keys, TrackMode::PingPong, false, false);
        assert_eq!(stage.tracks.len(), 1, "a track of the same property takes the other's place");
        stage.pass_tracks(90.0);
        assert!(close(get(&stage, PropId::X), 30.0), "on its way back");
        stage.pass_tracks(60.0);
        assert!(close(get(&stage, PropId::X), 30.0), "on its way there again");
        stage.stop_tracks(&pika(), Some(PropId::Y));
        assert_eq!(stage.tracks.len(), 1);
        stage.stop_tracks(&pika(), None);
        assert!(stage.tracks.is_empty() && close(get(&stage, PropId::X), 30.0), "stopped, it stays as it is");
    }

    #[test]
    fn each_kind_of_value_goes_its_way() {
        assert_eq!(between(Value::Point(Vec2::ZERO), Value::Point(Vec2::new(10.0, -20.0)), 0.5, false, false), Value::Point(Vec2::new(5.0, -10.0)));
        assert_eq!(between(Value::Colour([0, 0, 0, 0]), Value::Colour([255, 100, 50, 255]), 0.5, false, false), Value::Colour([128, 50, 25, 128]));
        assert!(close(between(Value::Number(350.0), Value::Number(10.0), 0.5, false, true), 360.0), "the short way round");
        assert!(close(between(Value::Number(350.0), Value::Number(10.0), 0.5, false, false), 180.0), "the whole way");
        assert!(close(between(Value::Number(350.0), Value::Number(10.0), 1.0, true, true), 10.0), "it lands where it was told");
        assert_eq!(between(Value::Bool(false), Value::Bool(true), 0.9, false, false), Value::Bool(false));
        assert_eq!(between(Value::Bool(false), Value::Bool(true), 1.0, true, false), Value::Bool(true));
        assert_eq!(between(Value::Nothing, Value::Colour([255, 0, 0, 255]), 0.5, false, false), Value::Nothing);
        assert_eq!(between(Value::Choice("normal"), Value::Choice("add"), 1.0, true, false), Value::Choice("add"));
    }

    #[test]
    fn an_angle_goes_the_short_way_unless_told() {
        let mut stage = stage();
        stage.set(&pika(), PropId::Angle, &number(350.0)).unwrap();
        stage.start_track(&pika(), PropId::Angle, &[key(60.0, number(10.0), Ease::Linear)], TrackMode::Once, false, false);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Angle), 360.0), "{:?}", get(&stage, PropId::Angle));
        stage.start_track(&pika(), PropId::Angle, &[key(0.0, number(0.0), Ease::Linear), key(60.0, number(720.0), Ease::Linear)], TrackMode::Once, false, true);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Angle), 360.0), "exact: two turns");
        stage.start_track(&pika(), PropId::Angle, &[key(60.0, Given::Plus(Value::Number(360.0)), Ease::Linear)], TrackMode::Once, false, false);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::Angle), 360.0 + 180.0), "a relative turn goes the whole way");
    }

    #[test]
    fn a_track_goes_in_its_node_time() {
        let mut stage = stage();
        stage.set(&pika(), PropId::Slowmo, &number(0.5)).unwrap();
        stage.start_track(&pika(), PropId::X, &[key(0.0, number(0.0), Ease::Linear), key(60.0, number(60.0), Ease::Linear)], TrackMode::Once, false, false);
        stage.pass_tracks(30.0);
        assert!(close(get(&stage, PropId::X), 15.0));
    }

    #[test]
    fn what_does_not_animate_or_is_not_there_is_refused() {
        let mut stage = stage();
        assert!(!stage.start_track(&pika(), PropId::Facing, &[key(10.0, Given::Is(Value::Choice("up")), Ease::Linear)], TrackMode::Once, false, false));
        assert!(!stage.start_track(&NodeRef::Named("nothing".to_string()), PropId::X, &[key(10.0, number(1.0), Ease::Linear)], TrackMode::Once, false, false));
        assert!(stage.tracks.is_empty());
        // (Its node gone, a track goes.)
        stage.start_track(&pika(), PropId::X, &[key(60.0, number(1.0), Ease::Linear)], TrackMode::Loop, false, false);
        stage.remove(Who::Monster(25));
        stage.pass_tracks(1.0);
        assert!(stage.tracks.is_empty());
    }

    #[test]
    fn settled_a_track_is_at_its_end() {
        let mut stage = stage();
        stage.start_track(&pika(), PropId::X, &[key(60.0, number(10.0), Ease::Linear)], TrackMode::Once, false, false);
        stage.start_track(&pika(), PropId::Y, &[key(60.0, number(10.0), Ease::Linear)], TrackMode::Loop, false, false);
        stage.settle();
        assert!(close(get(&stage, PropId::X), 10.0));
        assert_eq!(stage.tracks.len(), 1, "the loop goes on");
    }

    #[test]
    fn a_way_goes_at_an_even_speed() {
        // (Two pieces, one three times as long: halfway in time is halfway
        // along, not at the corner.)
        let way = TrackWay::new(&[Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::new(30.0, 10.0)], PathShape::Lines, 60.0, Ease::Linear, false);
        assert_eq!(way.along, vec![0.0, 30.0, 40.0]);
        assert_eq!(way.point(20.0), Vec2::new(20.0, 0.0));
        assert_eq!(way.point(35.0), Vec2::new(30.0, 5.0));
        assert_eq!((way.point(-5.0), way.point(99.0)), (Vec2::ZERO, Vec2::new(30.0, 10.0)));
        let mut stage = stage();
        stage.start_path(
            &pika(),
            &[Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::new(30.0, 10.0)],
            PathShape::Lines,
            60.0,
            Ease::Linear,
            TrackMode::Once,
            true,
            false,
        );
        stage.pass_tracks(30.0);
        assert_eq!(get(&stage, PropId::Position), Value::Point(Vec2::new(20.0, 0.0)));
        assert_eq!(stage.cast[&Who::Monster(25)].direction, towards(Vec2::ZERO, Vec2::X).unwrap(), "it faces the way it goes");
        stage.pass_tracks(30.0);
        assert_eq!(get(&stage, PropId::Position), Value::Point(Vec2::new(30.0, 10.0)));
        assert!(stage.tracks.is_empty());
    }

    #[test]
    fn curved_ways_go_through_or_by_their_points() {
        let points = [Vec2::ZERO, Vec2::new(50.0, 50.0), Vec2::new(100.0, 0.0)];
        let through = TrackWay::new(&points, PathShape::CatmullRom, 60.0, Ease::Linear, false);
        assert!(through.points.iter().any(|at| (*at - points[1]).length() < 1e-3), "a Catmull-Rom curve goes through its points");
        assert!(through.points.first() == Some(&points[0]) && (*through.points.last().unwrap() - points[2]).length() < 1e-3);
        assert!(*through.along.last().unwrap() > 141.0, "longer than straight from end to end");
        let by = TrackWay::new(&points, PathShape::Bezier, 60.0, Ease::Linear, false);
        let middle = by.points[by.points.len() / 2];
        assert!((middle - Vec2::new(50.0, 25.0)).length() < 1e-3, "a bezier goes by its controls: {middle}");
        // (Turned along its way, a picture.)
        let mut stage = stage();
        stage.direct_node(NodeBeat::Make { name: "moon".to_string(), sort: crate::beat::Sort::Group, at: crate::beat::Point::at(Vec2::ZERO) });
        let moon = NodeRef::Named("moon".to_string());
        stage.start_path(&moon, &[Vec2::ZERO, Vec2::new(0.0, 40.0)], PathShape::Lines, 60.0, Ease::Linear, TrackMode::Once, true, true);
        stage.pass_tracks(30.0);
        assert_eq!(stage.get(&moon, PropId::Position), Some(Value::Point(Vec2::new(0.0, 20.0))));
        assert!(close(stage.get(&moon, PropId::Angle).unwrap(), 90.0), "going down: turned a quarter clockwise");
    }

    #[test]
    fn a_relative_way_starts_where_it_is() {
        let mut stage = stage();
        stage.start_path(&pika(), &[Vec2::ZERO, Vec2::new(10.0, 0.0)], PathShape::Lines, 10.0, Ease::Linear, TrackMode::Once, false, true);
        stage.pass_tracks(10.0);
        assert_eq!(get(&stage, PropId::Position), Value::Point(Vec2::new(110.0, 100.0)));
    }

    #[test]
    fn the_same_track_plays_the_same() {
        let run = || {
            let mut stage = stage();
            let keys = [key(0.0, number(0.0), Ease::Linear), key(37.0, number(90.0), Ease::Out(EaseFamily::Elastic)), key(80.0, number(-30.0), Ease::Steps(3))];
            stage.start_track(&pika(), PropId::Angle, &keys, TrackMode::PingPong, false, true);
            (0..200)
                .map(|step| {
                    stage.pass_tracks(0.7 + (step % 3) as f32 * 0.3);
                    get(&stage, PropId::Angle)
                })
                .collect::<Vec<Value>>()
        };
        assert_eq!(run(), run());
    }
}
