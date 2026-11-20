//! The generic beats of the nodes ([`NodeBeat`]): a property of any node set
//! (`prop <node> <property> <value>`, `set` in films), a node of a film's
//! own made or taken away (`node <name> group|picture ...`), a property's track
//! (`track`), a node along a path (`trackpath`), tracks stopped
//! (`untrack`); played by the stage (`stage::tracks`).
//! What a property is, its name and its value, the registry says
//! (`crate::props`); who a node is, [`NodeRef`].

use super::declare::declare;
use std::fmt;

use super::parse::Fields;
use super::*;
use crate::props::{self, Given, PropId};
use crate::stage::Ease;
use crate::stage::node::Inherit;

/// A node of the stage, as a beat names it: someone or something of the
/// cast, a node by its name (a picture's, or one a film made), the caption,
/// a halo by its rank, the emitter on someone.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeRef {
    Who(Who),
    Named(String),
    Caption,
    Halo(u16),
    Emitter(Who),
}

impl NodeRef {
    /// As a line writes it: someone's code (`m25`, `o66.1`), `node:<name>`,
    /// `caption`, `halo:<n>`, `emitter:<who>`.
    pub fn parse(code: &str) -> Option<NodeRef> {
        if code == "caption" {
            return Some(NodeRef::Caption);
        }
        if let Some(name) = code.strip_prefix("node:") {
            return (!name.is_empty() && !name.contains([' ', '@', '+', ','])).then(|| NodeRef::Named(name.to_string()));
        }
        if let Some(rank) = code.strip_prefix("halo:") {
            return Some(NodeRef::Halo(rank.parse().ok()?));
        }
        if let Some(who) = code.strip_prefix("emitter:") {
            return Some(NodeRef::Emitter(Who::parse(who)?));
        }
        Who::parse(code).map(NodeRef::Who)
    }
}

/// As [`NodeRef::parse`] reads it back.
impl fmt::Display for NodeRef {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeRef::Who(who) => write!(out, "{who}"),
            NodeRef::Named(name) => write!(out, "node:{name}"),
            NodeRef::Caption => write!(out, "caption"),
            NodeRef::Halo(rank) => write!(out, "halo:{rank}"),
            NodeRef::Emitter(who) => write!(out, "emitter:{who}"),
        }
    }
}

/// What a node a film makes is (`node <name> <sort>`).
#[derive(Clone, Debug, PartialEq)]
pub enum Sort {
    /// Nothing but its place, its turn, its size: something to hang others
    /// on.
    Group,
    /// A picture from a file.
    Picture(String),
}

/// When a key of a track is: so many frames after the track starts, when
/// the music has played so many, or so many after a mark of the scene
/// (`mark name`). The director makes the last two the first, as the
/// track starts (`Director::keys_timed`).
#[derive(Clone, Debug, PartialEq)]
pub enum KeyTime {
    After(Frames),
    Music(Frames),
    Mark { name: String, plus: Frames },
}

impl KeyTime {
    /// As a line writes it, in one word: `120`, `music:720`, `mark:name+30`.
    pub fn word(&self) -> String {
        match self {
            KeyTime::After(frames) => frames.0.to_string(),
            KeyTime::Music(frames) => format!("music:{}", frames.0),
            KeyTime::Mark { name, plus } => format!("mark:{name}+{}", plus.0),
        }
    }

    pub fn read(word: &str) -> Option<KeyTime> {
        let frames = |text: &str| text.parse::<f32>().ok().filter(|frames| frames.is_finite()).map(Frames);
        if let Some(at) = word.strip_prefix("music:") {
            return Some(KeyTime::Music(frames(at)?));
        }
        if let Some(mark) = word.strip_prefix("mark:") {
            let (name, plus) = mark.split_once('+').unwrap_or((mark, "0"));
            if name.is_empty() {
                return None;
            }
            return Some(KeyTime::Mark { name: name.to_string(), plus: frames(plus)? });
        }
        frames(word).map(KeyTime::After)
    }
}

/// The ways a track's position may go along its points (`trackpath`):
/// straight from one to the next, by a Catmull-Rom curve through them
/// all, or by one bezier curve they are the controls of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PathShape {
    Lines,
    #[default]
    CatmullRom,
    Bezier,
}

impl PathShape {
    pub fn word(self) -> &'static str {
        match self {
            PathShape::Lines => "lines",
            PathShape::CatmullRom => "catmull",
            PathShape::Bezier => "bezier",
        }
    }

    /// By its word.
    pub fn named(word: &str) -> Option<PathShape> {
        Some(match word {
            "lines" => PathShape::Lines,
            "catmull" => PathShape::CatmullRom,
            "bezier" => PathShape::Bezier,
            _ => return None,
        })
    }
}

/// What a track does once at its last key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackMode {
    /// It stays there.
    #[default]
    Once,
    /// It starts over.
    Loop,
    /// It goes back, and again.
    PingPong,
}

/// A key of a track: when, that value (relative: to the key before), and
/// how it comes to it from the key before.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub at: KeyTime,
    pub value: Given<NodeRef>,
    pub ease: Ease,
}

impl TrackMode {
    pub fn word(self) -> &'static str {
        match self {
            TrackMode::Once => "once",
            TrackMode::Loop => "loop",
            TrackMode::PingPong => "pingpong",
        }
    }

    pub fn named(word: &str) -> Option<TrackMode> {
        Some(match word {
            "once" => TrackMode::Once,
            "loop" => TrackMode::Loop,
            "pingpong" => TrackMode::PingPong,
            _ => return None,
        })
    }
}

/// The generic beats of the nodes.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeBeat {
    /// A property of a node set, at once (see `Stage::set`): the value as
    /// written, relative, or a node's point.
    Set { node: NodeRef, prop: PropId, value: Given<NodeRef> },
    /// A property of a node going through its keys in time (a track: see
    /// `stage::tracks`), from what it is now if the first key is later than
    /// its start; each key added to what it was as it started if
    /// `relative`; an angle the short way round, unless `exact` (or the key
    /// is relative). It takes the place of a track of the node's property.
    Animate { node: NodeRef, prop: PropId, keys: Vec<Key>, mode: TrackMode, relative: bool, exact: bool },
    /// A node going along points in so many frames, at an even speed (its
    /// way's length), by that ease; turned the way it goes if `orient` (a
    /// body faces it); the points from where it is if `relative`.
    Path { node: NodeRef, points: Vec<Vec2>, shape: PathShape, frames: Frames, ease: Ease, mode: TrackMode, orient: bool, relative: bool },
    /// A node's tracks stopped (only that property's, if said): it stays
    /// as it is.
    Stop { node: NodeRef, prop: Option<PropId> },
    /// A node of the film's own, by its name, at a point.
    Make { name: String, sort: Sort, at: Point },
    /// A node of the film's own taken away.
    Unmake(String),
    /// A node hung on another (`attach a b [dx dy] [inherit ...]`): it
    /// goes with it, so far from its position (none: where it is now), in
    /// its turned and sized frame, taking from it what `inherit` says.
    Attach { node: NodeRef, to: NodeRef, offset: Option<Vec2>, inherit: Inherit },
    /// A node let go of (`detach a`): it stays where it is.
    Detach(NodeRef),
}

/// A node's own name, as `node` takes it: a plain word (a part of a rig:
/// `robot.bras`).
fn name(field: &str) -> Option<String> {
    let plain = !field.is_empty() && field.chars().all(|letter| letter.is_alphanumeric() || letter == '_' || letter == '.');
    plain.then(|| field.to_string())
}

declare! {
    NodeBeat;
    hand: "attach", "detach", "node", "prop", "track", "trackpath", "untrack";
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new(
        "attach",
        &["node:word parent:word [dx:number dy:number] [inherit pos angle:number scale:number opacity colour:word]"],
        "the node hangs from its parent: it goes where the parent puts it, at dx dy from its position in its turned and scaled frame (without dx dy: where it is), its pivot kept; it takes from it what `inherit` says (without: everything); a parent's parent counts too; neither the caption nor an emitter; a body carried or on a back is let go; a body attached to a body is drawn just in front of it (without a `z` of its own)",
    ),
    Form::new("detach", &["node:word"], "it no longer hangs from anything and stays where it is"),
    Form::new(
        "node",
        &["name:word group [x:coordinate y:coordinate]", "name:word picture file:word [x:coordinate y:coordinate]", "name:word off"],
        "a node of your own: a group, just its place, to hang other nodes on; a picture from a file, put there; `off`: removes it",
    ),
    Form::new(
        "prop",
        &["node:word property:word value:number"],
        "a property of any node, at once: `prop m25 angle 45`, `prop m25 opacity 50%`, `prop m25 tint #ff8800`, `prop m25 position @m7+10,-4`, `prop m25 scale x1.5`, `prop m25 angle +30`, `prop m25 angle default`. The node: someone (`m25`, `o66.3`), `node:<name>` (a picture, a node made by `node`), `caption`, `halo:<rank>`, `emitter:<who>`. The properties, their types, units, defaults and advised ranges: the \"Properties\" table of `film/FILMS.md` (the registry, `engine/src/props/`); a value outside the advised range plays as is, with an `R-HORS-PLAGE` remark; an impossible one (a negative size) is an `E-VALEUR-IMPOSSIBLE` error",
    ),
    Form::new(
        "track",
        &["node:word property:word [rest...]"],
        "a track: the property goes through its keys (any animatable property of the registry). `<t>`: so many frames after the start (`120`), `music:720` (when the music has played so much), `mark:<name>+30` (so much after `set _mark_<name> clock`); a key already passed: at once. The first key after 0: the track starts from what the property is. `<value>` as for `prop`; `+30`: added to the key before; `relative`: each key added to what it was at the start. `<curve>`: how it comes to this key (`linear`, `smooth`, `in_quad`, `out_back`, `inout_sine`, `steps(4)`, `bezier(0.4,0,0.2,1)`; the list in `film/FILMS.md`). An angle takes the shortest way, unless `exact` or a relative key. `loop`: it starts again; `pingpong`: it comes back, and sets off again. A new track of the same property of the same node takes the other's place. The node's time counts (`slowmo`), and `tempo`",
    ),
    Form::new(
        "trackpath",
        &["node:word frames:duration linescatmullbezier:word curve:word [rest...]"],
        "the node follows these points at an even speed (by the length of the path), in so many frames, by this curve: in straight lines, in a Catmull-Rom curve through all the points, or in one Bézier curve whose controls they are; `orient`: turned the way it goes (a body: it looks that way); `relative`: the points from where it is",
    ),
    Form::new("untrack", &["node:word [property:word]"], "its tracks stop (only that property's): it stays as it is"),
];

pub(super) fn read(fields: &Fields) -> Option<NodeBeat> {
    if let Some(beat) = NodeBeat::declared(fields) {
        return Some(beat);
    }
    // (A node's values have a grammar of their own, the properties'
    // (`props::read`): read in their words, a number as it shows.)
    let line = &fields.spelled();
    Some(match line.first()?.as_str() {
        // (`prop <node> <property> <value>`: the value as the grammar has it,
        // all the rest of the line.)
        "prop" => {
            let prop = props::find(line.get(2)?)?;
            let (value, used) = props::read(prop, line.get(3..)?).ok()?;
            if 3 + used != line.len() {
                return None;
            }
            NodeBeat::Set { node: NodeRef::parse(line.get(1)?)?, prop: prop.id, value: value.map(|node| NodeRef::parse(&node))? }
        }
        "node" => {
            let name = name(line.get(1)?)?;
            // (Where, if said: else at the top left.)
            let at = |from: usize| match line.len() - from {
                0 => Some(Point::at(Vec2::ZERO)),
                2 => fields.point(from),
                _ => None,
            };
            match line.get(2)?.as_str() {
                "off" if line.len() == 3 => NodeBeat::Unmake(name),
                "group" => NodeBeat::Make { name, sort: Sort::Group, at: at(3)? },
                "picture" => NodeBeat::Make { name, sort: Sort::Picture(line.get(3)?.clone()), at: at(4)? },
                _ => return None,
            }
        }
        // (`attach <node> <to> [dx dy] [inherit pos angle scale opacity colour]`.)
        "attach" => {
            let (node, to) = (NodeRef::parse(line.get(1)?)?, NodeRef::parse(line.get(2)?)?);
            let flags = line.iter().position(|word| word == "inherit");
            let numbers = &line[3..flags.unwrap_or(line.len())];
            let offset = match numbers {
                [] => None,
                [x, y] => Some(Vec2::new(x.parse().ok()?, y.parse().ok()?)),
                _ => return None,
            };
            let inherit = match flags {
                Some(at) => Inherit::of_words(&line[at + 1..])?,
                None => Inherit::default(),
            };
            NodeBeat::Attach { node, to, offset, inherit }
        }
        "detach" if line.len() == 2 => NodeBeat::Detach(NodeRef::parse(line.get(1)?)?),
        // (`track <node> <prop> [loop|pingpong] [relative] [exact] / <at> <value> <ease> / ...`.)
        "track" => {
            let (node, prop) = (NodeRef::parse(line.get(1)?)?, props::find(line.get(2)?)?);
            let first = line.iter().position(|word| word == "/")?;
            let (mut mode, mut relative, mut exact) = (TrackMode::Once, false, false);
            for word in line.get(3..first)? {
                match word.as_str() {
                    "relative" => relative = true,
                    "exact" => exact = true,
                    other => mode = TrackMode::named(other)?,
                }
            }
            let mut keys = Vec::new();
            for key in line[first + 1..].split(|word| word == "/") {
                let at = KeyTime::read(key.first()?)?;
                let (value, used) = props::read(prop, key.get(1..)?).ok()?;
                let (ease, eased) = Ease::read(key.get(1 + used..)?)?;
                if 1 + used + eased != key.len() {
                    return None;
                }
                keys.push(Key { at, value: value.map(|node| NodeRef::parse(&node))?, ease });
            }
            NodeBeat::Animate { node, prop: prop.id, keys, mode, relative, exact }
        }
        // (`trackpath <node> <frames> <shape> <ease> [loop|pingpong] [orient] [relative] / x y / x y ...`.)
        "trackpath" => {
            let node = NodeRef::parse(line.get(1)?)?;
            let frames = Frames(line.get(2)?.parse().ok().filter(|frames: &f32| frames.is_finite())?);
            let shape = PathShape::named(line.get(3)?)?;
            let (ease, used) = Ease::read(line.get(4..)?)?;
            let first = line.iter().position(|word| word == "/")?;
            let (mut mode, mut orient, mut relative) = (TrackMode::Once, false, false);
            for word in line.get(4 + used..first)? {
                match word.as_str() {
                    "orient" => orient = true,
                    "relative" => relative = true,
                    other => mode = TrackMode::named(other)?,
                }
            }
            let mut points = Vec::new();
            for point in line[first + 1..].split(|word| word == "/") {
                match point {
                    [x, y] => points.push(Vec2::new(x.parse().ok()?, y.parse().ok()?)),
                    _ => return None,
                }
            }
            if points.len() < 2 || !points.iter().all(|at| at.is_finite()) {
                return None;
            }
            NodeBeat::Path { node, points, shape, frames, ease, mode, orient, relative }
        }
        "untrack" => NodeBeat::Stop {
            node: NodeRef::parse(line.get(1)?)?,
            prop: match line.len() {
                2 => None,
                3 => Some(props::find(line.get(2)?)?.id),
                _ => return None,
            },
        },
        _ => return None,
    })
}
