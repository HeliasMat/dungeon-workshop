//! Everything the stage shows as one model, a node ([`Node`]): its name, its
//! kind, how it is placed ([`Transform`]), how it looks ([`Looks`]), its rank
//! (`z`), whether it shows, who it hangs on and what of it it takes from
//! there ([`Inherit`]). Its properties are the registry's
//! (`crate::props`), each read by [`Stage::get`] and set by [`Stage::set`].
//!
//! The bodies, the things, the pictures, the caption, the halos and the
//! emitters keep what they always had (an `Actor`'s position, its opacity,
//! its size, its colour, its flip; a picture's point; a halo's width): a
//! node wraps them. `Stage::nodes` keeps only what they had not (an angle,
//! a skew, a pivot, a blend, a `z`...), the identity until told, and what a
//! node of a film's own is wholly (`node n group`). [`Stage::node`]
//! gives a node as it is now, the two put together: what is drawn reads
//! it (workstream 3), what animates sets it by its properties (workstream
//! 4), whatever its kind.

use glam::{Affine2, Mat2};

use super::*;
pub use crate::base::Inherit;
use crate::beat::{NodeBeat, NodeRef, Sort};
use crate::props::{Blend, Given, NodeKind, PropId, Rgba, Value};

/// How a node is placed: where, turned, sized and slanted about its pivot,
/// flipped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub pos: Vec2,
    pub angle_deg: f32,
    pub scale: Vec2,
    pub skew: Vec2,
    /// The point it turns and grows about, from its position.
    pub pivot: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Default for Transform {
    /// Where it is, as it is.
    fn default() -> Self {
        Transform { pos: Vec2::ZERO, angle_deg: 0.0, scale: Vec2::ONE, skew: Vec2::ZERO, pivot: Vec2::ZERO, flip_x: false, flip_y: false }
    }
}

impl Transform {
    /// Its flip, size, slant and turn, in that order, about its pivot: a
    /// map on points from its position (the position itself left out).
    /// What is drawn of it goes through it; the turn is clockwise on the
    /// screen, the slants those of `skew_x` (sideways, as the rows
    /// go down) and `skew_y`.
    pub fn about_pivot(&self) -> Affine2 {
        let flip = Vec2::new(if self.flip_x { -1.0 } else { 1.0 }, if self.flip_y { -1.0 } else { 1.0 });
        let slant = Mat2::from_cols(Vec2::new(1.0, self.skew.y.to_radians().tan()), Vec2::new(self.skew.x.to_radians().tan(), 1.0));
        let linear = Mat2::from_angle(self.angle_deg.to_radians()) * slant * Mat2::from_diagonal(self.scale * flip);
        Affine2::from_translation(self.pivot) * Affine2::from_mat2(linear) * Affine2::from_translation(-self.pivot)
    }

    /// Not turned, slanted, flipped up and down, nor sized one way more
    /// than the other: drawn as a sprite always was.
    pub fn is_upright(&self) -> bool {
        self.angle_deg.rem_euclid(360.0) == 0.0 && self.skew == Vec2::ZERO && !self.flip_y && self.scale.x == self.scale.y && self.scale.x > 0.0
    }

    /// The box that holds a rectangle (its low and high corners, from the
    /// node's position) once it goes through [`Transform::about_pivot`].
    pub fn bounds(&self, low: Vec2, high: Vec2) -> (Vec2, Vec2) {
        let map = self.about_pivot();
        let corners = [low, Vec2::new(high.x, low.y), Vec2::new(low.x, high.y), high].map(|corner| map.transform_point2(corner));
        (corners.iter().fold(Vec2::splat(f32::INFINITY), |low, at| low.min(*at)), corners.iter().fold(Vec2::splat(f32::NEG_INFINITY), |high, at| high.max(*at)))
    }
}

/// An effect drawn on a node, as [`Effects::list`] gives those it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeEffect {
    /// A line of a colour about its shape, so wide (pixels of the scene).
    Outline { colour: Rgba, width: f32 },
    /// A glow of a colour about it, so wide, in three hard bands.
    Glow { colour: Rgba, width: f32 },
    /// Its shape in a colour, so far off, under it.
    DropShadow { colour: Rgba, offset: Vec2 },
    /// So much of its pixels gone (0 to 1), drawn by its seed.
    Dissolve { amount: f32, seed: f32 },
    /// Its rows swayed so far (pixels of its picture), every so many rows,
    /// so many times a second.
    Wave { amplitude: f32, length: f32, speed: f32 },
    /// A colour of its picture shown as another.
    PaletteSwap { from: Rgba, to: Rgba },
    /// All of it gone towards a colour, so much (a silhouette: `fill`).
    Flash { colour: Rgba, amount: f32 },
}

impl NodeEffect {
    /// Its name, as `state --json` says it.
    pub fn name(&self) -> &'static str {
        match self {
            NodeEffect::Outline { .. } => "outline",
            NodeEffect::Glow { .. } => "glow",
            NodeEffect::DropShadow { .. } => "drop_shadow",
            NodeEffect::Dissolve { .. } => "dissolve",
            NodeEffect::Wave { .. } => "wave",
            NodeEffect::PaletteSwap { .. } => "palette",
            NodeEffect::Flash { .. } => "flash",
        }
    }
}

/// The effects of a node, each by its properties (see the registry:
/// `outline`, `glow`, `drop_shadow`, `dissolve`, `wave`, `swap_from`,
/// `flash` and theirs): those with no colour, or nothing to do, are off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    pub outline: Option<Rgba>,
    pub outline_width: f32,
    pub glow: Option<Rgba>,
    pub glow_width: f32,
    pub shadow: Option<Rgba>,
    pub shadow_offset: Vec2,
    pub dissolve: f32,
    pub seed: f32,
    pub wave: f32,
    pub wave_length: f32,
    pub wave_speed: f32,
    pub swap_from: Option<Rgba>,
    pub swap_to: Option<Rgba>,
    pub flash: Option<Rgba>,
    pub flash_amount: f32,
}

impl Default for Effects {
    /// None of them, each at the registry's default.
    fn default() -> Self {
        Effects {
            outline: None,
            outline_width: 1.0,
            glow: None,
            glow_width: 3.0,
            shadow: None,
            shadow_offset: Vec2::new(2.0, 2.0),
            dissolve: 0.0,
            seed: 0.0,
            wave: 0.0,
            wave_length: 16.0,
            wave_speed: 1.0,
            swap_from: None,
            swap_to: None,
            flash: None,
            flash_amount: 1.0,
        }
    }
}

impl Effects {
    /// Those it has, in the order they are drawn: under it, then on it.
    pub fn list(&self) -> Vec<NodeEffect> {
        let mut all = Vec::new();
        all.extend(self.shadow.map(|colour| NodeEffect::DropShadow { colour, offset: self.shadow_offset }));
        all.extend(self.glow.filter(|_| self.glow_width > 0.0).map(|colour| NodeEffect::Glow { colour, width: self.glow_width }));
        all.extend(self.outline.filter(|_| self.outline_width > 0.0).map(|colour| NodeEffect::Outline { colour, width: self.outline_width }));
        if self.dissolve > 0.0 {
            all.push(NodeEffect::Dissolve { amount: self.dissolve, seed: self.seed });
        }
        if self.wave != 0.0 {
            all.push(NodeEffect::Wave { amplitude: self.wave, length: self.wave_length, speed: self.wave_speed });
        }
        if let (Some(from), Some(to)) = (self.swap_from, self.swap_to) {
            all.push(NodeEffect::PaletteSwap { from, to });
        }
        all.extend(self.flash.filter(|_| self.flash_amount > 0.0).map(|colour| NodeEffect::Flash { colour, amount: self.flash_amount }));
        all
    }

    pub fn is_none(&self) -> bool {
        self.list().is_empty()
    }
}

/// How a node looks: how much of it shows, the colour that multiplies its
/// own and the one added to it, a colour filling its shape, how it mixes
/// with what is under it, its effects.
#[derive(Clone, Debug, PartialEq)]
pub struct Looks {
    pub opacity: f32,
    pub tint: Rgba,
    pub add: [u8; 3],
    pub fill: Option<Rgba>,
    pub blend: Blend,
    /// Turned or grown, smoothed (bilinear) rather than in sharp pixels.
    pub smooth: bool,
    pub effects: Effects,
}

impl Default for Looks {
    /// As it is.
    fn default() -> Self {
        Looks { opacity: 1.0, tint: [255; 4], add: [0; 3], fill: None, blend: Blend::Normal, smooth: false, effects: Effects::default() }
    }
}

impl Looks {
    /// Nothing added, filled, mixed otherwise or smoothed: drawn as a
    /// sprite always was (its tint and opacity are its owner's).
    pub fn is_plain(&self) -> bool {
        self.add == [0; 3] && self.fill.is_none() && self.blend == Blend::Normal && !self.smooth && self.effects.is_none()
    }
}

/// What a node takes from those it hangs on, all the way up: their turn,
/// their size, how much of them shows, their tint (of 1). Added to its
/// own when it is drawn ([`Stage::node`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hung {
    pub angle_deg: f32,
    pub scale: Vec2,
    pub opacity: f32,
    pub tint: [f32; 4],
}

impl Default for Hung {
    /// Hung on nothing.
    fn default() -> Self {
        Hung { angle_deg: 0.0, scale: Vec2::ONE, opacity: 1.0, tint: [1.0; 4] }
    }
}

/// How many nodes up a chain of parents is followed: past it (a loop the
/// checks did not see), no further.
const DEEPEST: usize = 64;

/// Something the stage shows, as one model.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub kind: NodeKind,
    pub transform: Transform,
    pub looks: Looks,
    /// Its rank among the others (none: as its kind has it, today's
    /// order: the place, the bodies by their feet, what is over them).
    pub z: Option<f32>,
    pub visible: bool,
    /// Who it hangs on (none: the scene), and what it takes from them
    /// (`attach`); where it hangs, from its parent's position, in its
    /// parent's turned and sized frame.
    pub parent: Option<NodeRef>,
    pub inherit: Inherit,
    pub offset: Vec2,
}

impl Node {
    /// A node just made: where it is, as it is.
    pub fn new(name: impl Into<String>, kind: NodeKind) -> Node {
        Node {
            name: name.into(),
            kind,
            transform: Transform::default(),
            looks: Looks::default(),
            z: None,
            visible: true,
            parent: None,
            inherit: Inherit::default(),
            offset: Vec2::ZERO,
        }
    }
}

/// Why a property is not set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetError {
    /// No such node on the stage.
    NoNode,
    /// Its kind has no such property.
    NotOfKind(NodeKind),
    /// The value is not of the property's type (or cannot be worked out:
    /// a relative colour, a node's point that is not there).
    WrongValue,
    /// The value is impossible (see `Prop::check`).
    Impossible,
}

impl NodeRef {
    /// Its name, as a film reads it.
    pub fn name(&self) -> String {
        match self {
            NodeRef::Who(who) => who.to_string(),
            NodeRef::Named(name) => name.clone(),
            NodeRef::Caption => "caption".to_string(),
            NodeRef::Halo(rank) => format!("halo:{rank}"),
            NodeRef::Emitter(who) => format!("emitter:{who}"),
        }
    }
}

/// A colour of the stage's (of 255, as floats), as a node's.
fn rgba(colour: [f32; 3], alpha: u8) -> Rgba {
    let [r, g, b] = colour.map(|part| part.round().clamp(0.0, 255.0) as u8);
    [r, g, b, alpha]
}

impl Stage {
    /// The kind of a node, if it is on the stage.
    pub fn kind_of(&self, node: &NodeRef) -> Option<NodeKind> {
        match node {
            NodeRef::Who(who) if self.cast.contains_key(who) => Some(if who.is_thing() { NodeKind::Thing } else { NodeKind::Body }),
            NodeRef::Who(_) => None,
            NodeRef::Named(name) if self.words.pictures.iter().any(|picture| picture.name == *name) => Some(NodeKind::Picture),
            NodeRef::Named(_) => self.nodes.get(node).filter(|made| made.kind == NodeKind::Group).map(|made| made.kind),
            NodeRef::Caption => self.words.caption.is_some().then_some(NodeKind::Caption),
            NodeRef::Halo(rank) => self.light.halos.get(usize::from(*rank)).map(|_| NodeKind::Halo),
            NodeRef::Emitter(who) => self.light.emitters.iter().any(|emitter| emitter.who == *who).then_some(NodeKind::Emitter),
        }
    }

    /// Every node on the stage, the cast first.
    pub fn node_refs(&self) -> Vec<NodeRef> {
        let mut all: Vec<NodeRef> = self.cast.keys().map(|who| NodeRef::Who(*who)).collect();
        all.extend(self.words.pictures.iter().map(|picture| NodeRef::Named(picture.name.clone())));
        all.extend(self.nodes.iter().filter(|(_, made)| made.kind == NodeKind::Group).map(|(node, _)| node.clone()));
        all.extend(self.words.caption.as_ref().map(|_| NodeRef::Caption));
        all.extend((0..self.light.halos.len()).filter_map(|rank| u16::try_from(rank).ok()).map(NodeRef::Halo));
        all.extend(self.light.emitters.iter().map(|emitter| NodeRef::Emitter(emitter.who)));
        all.dedup();
        all
    }

    /// What the stage keeps of a node past what its owner has.
    fn kept(&self, node: &NodeRef, kind: NodeKind) -> Node {
        let mut kept = self.nodes.get(node).cloned().unwrap_or_else(|| Node::new(node.name(), kind));
        kept.kind = kind;
        kept
    }

    /// Where a halo or the view is: its point, or who it is on.
    fn cam_point(&self, on: &Cam) -> Vec2 {
        match on {
            Cam::At(at) => at.vec2(),
            Cam::Who(who) => self.cast.get(who).map_or(Vec2::ZERO, |actor| actor.position),
        }
    }

    /// A node as it is drawn now: as it is alone ([`Stage::node_alone`]),
    /// with what it takes from those it hangs on ([`Stage::hung`]): their
    /// turn added to its own, their size, opacity and tint multiplying its
    /// own. (Its position is where they put it already: see
    /// `Stage::attached_along`.)
    pub fn node(&self, node: &NodeRef) -> Option<Node> {
        let mut shown = self.node_alone(node)?;
        if shown.parent.is_some() {
            let hung = self.hung(node);
            shown.transform.angle_deg += hung.angle_deg;
            shown.transform.scale *= hung.scale;
            shown.looks.opacity *= hung.opacity;
            for (part, by) in shown.looks.tint.iter_mut().zip(hung.tint) {
                *part = (*part as f32 * by).round().clamp(0.0, 255.0) as u8;
            }
        }
        Some(shown)
    }

    /// What a node takes from those it hangs on, up the chain: each takes
    /// from its parent what it inherits (its parent's own, and what its
    /// parent took). Nothing for a node on no one.
    pub fn hung(&self, node: &NodeRef) -> Hung {
        let mut chain = Vec::new();
        let mut at = node.clone();
        while chain.len() < DEEPEST {
            let Some(kept) = self.nodes.get(&at) else { break };
            let Some(parent) = kept.parent.clone() else { break };
            chain.push((kept.inherit, parent.clone()));
            at = parent;
        }
        // (From the top down: each parent as it is with what it took.)
        let mut hung = Hung::default();
        for (inherit, parent) in chain.into_iter().rev() {
            let Some(alone) = self.node_alone(&parent) else {
                hung = Hung::default();
                continue;
            };
            let (transform, looks) = (alone.transform, alone.looks);
            let whole = Hung {
                angle_deg: hung.angle_deg + transform.angle_deg,
                scale: hung.scale * transform.scale,
                opacity: hung.opacity * looks.opacity,
                tint: std::array::from_fn(|at| hung.tint[at] * looks.tint[at] as f32 / 255.0),
            };
            hung = Hung {
                angle_deg: if inherit.angle { whole.angle_deg } else { 0.0 },
                scale: if inherit.scale { whole.scale } else { Vec2::ONE },
                opacity: if inherit.opacity { whole.opacity } else { 1.0 },
                tint: if inherit.colour { whole.tint } else { [1.0; 4] },
            };
        }
        hung
    }

    /// A node as it is now: what the stage keeps of it, with what its
    /// owner has (a body's position, size, opacity, colour, flip...),
    /// nothing of those it hangs on.
    pub fn node_alone(&self, node: &NodeRef) -> Option<Node> {
        let kind = self.kind_of(node)?;
        let mut shown = self.kept(node, kind);
        let (transform, looks) = (&mut shown.transform, &mut shown.looks);
        match node {
            NodeRef::Who(who) => {
                let actor = &self.cast[who];
                transform.pos += actor.position;
                transform.angle_deg += actor.shown_turn();
                transform.scale *= actor.looks.scale;
                transform.flip_x ^= actor.flipped;
                looks.opacity *= actor.looks.opacity;
                looks.tint = rgba(actor.colour, looks.tint[3]);
                shown.visible &= actor.attributes & attribute::HIDDEN == 0;
            }
            NodeRef::Named(name) => {
                if let Some(picture) = self.words.pictures.iter().find(|picture| picture.name == *name) {
                    transform.pos += picture.at;
                }
            }
            NodeRef::Halo(rank) => transform.pos += self.cam_point(&self.light.halos[usize::from(*rank)].on),
            NodeRef::Emitter(who) => transform.pos += self.cast.get(who).map_or(Vec2::ZERO, |actor| actor.position),
            NodeRef::Caption => {}
        }
        Some(shown)
    }

    /// A property of a node as it is now; none if the node is not there or
    /// its kind has no such property. Its own: where it is, but its turn,
    /// size, opacity and tint without those of whom it hangs on (drawn,
    /// they are added: [`Stage::node`]).
    pub fn get(&self, node: &NodeRef, prop: PropId) -> Option<Value> {
        let shown = self.node_alone(node)?;
        if !prop.prop().fits(shown.kind) {
            return None;
        }
        let actor = match node {
            NodeRef::Who(who) => self.cast.get(who),
            _ => None,
        };
        let (transform, looks) = (&shown.transform, &shown.looks);
        Some(match prop {
            PropId::Position => Value::Point(transform.pos),
            PropId::X => Value::Number(transform.pos.x),
            PropId::Y => Value::Number(transform.pos.y),
            PropId::Angle => Value::Number(transform.angle_deg),
            PropId::Scale | PropId::ScaleX => Value::Number(transform.scale.x),
            PropId::ScaleY => Value::Number(transform.scale.y),
            PropId::SkewX => Value::Number(transform.skew.x),
            PropId::SkewY => Value::Number(transform.skew.y),
            PropId::Pivot => Value::Point(transform.pivot),
            PropId::FlipX => Value::Bool(transform.flip_x),
            PropId::FlipY => Value::Bool(transform.flip_y),
            PropId::Opacity => Value::Number(looks.opacity),
            PropId::Tint => Value::Colour(looks.tint),
            PropId::Add => Value::Colour([looks.add[0], looks.add[1], looks.add[2], 255]),
            PropId::Fill => looks.fill.map_or(Value::Nothing, Value::Colour),
            PropId::Blend => Value::Choice(looks.blend.name()),
            PropId::Smooth => Value::Bool(looks.smooth),
            PropId::Z => shown.z.map_or(Value::Nothing, Value::Number),
            PropId::Visible => Value::Bool(shown.visible),
            PropId::Height => Value::Number(actor?.air.height),
            PropId::Sink => Value::Number(actor?.sunk),
            PropId::Depth => Value::Number(actor?.looks.depth),
            PropId::Slowmo => Value::Number(actor?.looks.time),
            PropId::AnimSpeed => Value::Number(actor?.looks.speed),
            PropId::Shadow => Value::Bool(!actor?.shadowless),
            PropId::Facing => Value::Choice(Direction::from_number(actor?.direction).name()),
            PropId::Radius => match node {
                NodeRef::Halo(rank) => Value::Number(self.light.halos[usize::from(*rank)].radius),
                _ => return None,
            },
            PropId::Colour => match node {
                NodeRef::Halo(rank) => Value::Colour(rgba(self.light.halos[usize::from(*rank)].colour.map(f32::from), 255)),
                NodeRef::Emitter(who) => {
                    let colour = self.light.emitters.iter().find(|emitter| emitter.who == *who)?.colour;
                    Value::Colour(rgba(colour.unwrap_or([255; 3]).map(f32::from), 255))
                }
                _ => return None,
            },
            PropId::Rate => match node {
                NodeRef::Emitter(who) => Value::Number(60.0 / self.light.emitters.iter().find(|emitter| emitter.who == *who)?.between.max(f32::MIN_POSITIVE)),
                _ => return None,
            },
            effect => {
                let effects = &looks.effects;
                let colour = |colour: Option<Rgba>| colour.map_or(Value::Nothing, Value::Colour);
                match effect {
                    PropId::Outline => colour(effects.outline),
                    PropId::OutlineSize => Value::Number(effects.outline_width),
                    PropId::Glow => colour(effects.glow),
                    PropId::GlowSize => Value::Number(effects.glow_width),
                    PropId::DropShadow => colour(effects.shadow),
                    PropId::ShadowOffset => Value::Point(effects.shadow_offset),
                    PropId::Dissolve => Value::Number(effects.dissolve),
                    PropId::Seed => Value::Number(effects.seed),
                    PropId::Wave => Value::Number(effects.wave),
                    PropId::WaveLength => Value::Number(effects.wave_length),
                    PropId::WaveSpeed => Value::Number(effects.wave_speed),
                    PropId::SwapFrom => colour(effects.swap_from),
                    PropId::SwapTo => colour(effects.swap_to),
                    PropId::Flash => colour(effects.flash),
                    PropId::FlashAmount => Value::Number(effects.flash_amount),
                    _ => return None,
                }
            }
        })
    }

    /// Sets a property of a node, as written: as it is, added to what it is,
    /// multiplied, a node's point, the default. No clamp: what the registry
    /// says is impossible is refused, the rest played as asked.
    pub fn set(&mut self, node: &NodeRef, prop: PropId, given: &Given<NodeRef>) -> Result<(), SetError> {
        let kind = self.kind_of(node).ok_or(SetError::NoNode)?;
        let registered = prop.prop();
        if !registered.fits(kind) {
            return Err(SetError::NotOfKind(kind));
        }
        let now = || self.get(node, prop).ok_or(SetError::WrongValue);
        let value = match given {
            Given::Is(value) => *value,
            Given::Default => registered.default,
            Given::Plus(by) => now()?.plus(*by).ok_or(SetError::WrongValue)?,
            Given::Times(by) => now()?.times(*by).ok_or(SetError::WrongValue)?,
            Given::At { node: other, by } => match self.get(other, PropId::Position) {
                Some(Value::Point(at)) => Value::Point(at + *by),
                _ => return Err(SetError::WrongValue),
            },
        };
        registered.check(&value).map_err(|_| SetError::Impossible)?;
        // (The default of a node's place is where its owner put it: kept.)
        if *given == Given::Default && matches!(prop, PropId::Position | PropId::X | PropId::Y | PropId::Facing) {
            return Ok(());
        }
        // (Hung on someone, its place is where it hangs on them.)
        if matches!(prop, PropId::Position | PropId::X | PropId::Y)
            && let Some(kept) = self.nodes.get(node).filter(|kept| kept.inherit.pos).cloned()
            && let Some(map) = kept.parent.as_ref().and_then(|parent| self.map_of(parent))
        {
            let now = self.node_alone(node).ok_or(SetError::NoNode)?.transform.pos;
            let there = match prop {
                PropId::X => Vec2::new(value.number().ok_or(SetError::WrongValue)?, now.y),
                PropId::Y => Vec2::new(now.x, value.number().ok_or(SetError::WrongValue)?),
                _ => value.point().ok_or(SetError::WrongValue)?,
            };
            let pivot = kept.transform.pivot;
            let offset = map.inverse().transform_point2(there + pivot) - pivot;
            self.nodes.insert(node.clone(), Node { offset, ..kept });
            self.hang_now(node);
            return Ok(());
        }
        self.put_value(node, kind, prop, value)
    }

    /// Where a node's points go, drawn: from its position, turned, sized
    /// and slanted about its pivot (as those it hangs on have it too).
    pub fn map_of(&self, node: &NodeRef) -> Option<Affine2> {
        let shown = self.node(node)?;
        Some(Affine2::from_translation(shown.transform.pos) * shown.transform.about_pivot())
    }

    /// How many nodes up its chain of parents a node hangs.
    pub fn depth_of(&self, node: &NodeRef) -> usize {
        let mut at = node.clone();
        let mut depth = 0;
        while depth < DEEPEST {
            match self.nodes.get(&at).and_then(|kept| kept.parent.clone()) {
                Some(parent) => (at, depth) = (parent, depth + 1),
                None => break,
            }
        }
        depth
    }

    /// Whether `node` is `other` or hangs on it, however far up.
    pub fn hangs_on(&self, node: &NodeRef, other: &NodeRef) -> bool {
        let mut at = node.clone();
        for _ in 0..=DEEPEST {
            if at == *other {
                return true;
            }
            match self.nodes.get(&at).and_then(|kept| kept.parent.clone()) {
                Some(parent) => at = parent,
                None => return false,
            }
        }
        true
    }

    /// Hangs a node on another, so far from its position (none: where it
    /// is now), taking from it what `inherit` says; false if it cannot (one
    /// of them is not there, it would hang on itself, a caption or an
    /// emitter does not move by itself). One carried or riding is let go
    /// of: it hangs now.
    pub fn attach(&mut self, node: &NodeRef, to: &NodeRef, offset: Option<Vec2>, inherit: Inherit) -> bool {
        let (Some(kind), Some(_)) = (self.kind_of(node), self.kind_of(to)) else {
            return false;
        };
        if matches!(kind, NodeKind::Caption | NodeKind::Emitter) || self.hangs_on(to, node) {
            return false;
        }
        let Some(map) = self.map_of(to) else {
            return false;
        };
        let mut kept = self.kept(node, kind);
        let pivot = kept.transform.pivot;
        let here = self.node_alone(node).map_or(Vec2::ZERO, |alone| alone.transform.pos);
        kept.offset = offset.unwrap_or_else(|| map.inverse().transform_point2(here + pivot) - pivot);
        (kept.parent, kept.inherit) = (Some(to.clone()), inherit);
        self.nodes.insert(node.clone(), kept);
        if let NodeRef::Who(who) = node {
            self.with(*who, |actor| (actor.habits.carried_by, actor.habits.held, actor.habits.rides) = (None, false, None));
        }
        self.hang_now(node);
        true
    }

    /// Lets a node go: it stays where it is, with its own turn and size.
    pub fn detach(&mut self, node: &NodeRef) {
        if let Some(kept) = self.nodes.get_mut(node) {
            (kept.parent, kept.offset) = (None, Vec2::ZERO);
        }
    }

    /// Those hung on others go where they hang, the higher first: a child
    /// of a child goes where its parent was just put.
    pub(super) fn attached_along(&mut self) {
        let mut hanging: Vec<(usize, NodeRef)> =
            self.nodes.iter().filter(|(_, kept)| kept.parent.is_some()).map(|(node, _)| (self.depth_of(node), node.clone())).collect();
        if hanging.is_empty() {
            return;
        }
        hanging.sort();
        for (_, node) in hanging {
            self.hang_now(&node);
        }
    }

    /// A node put where it hangs on its parent now: its position the
    /// parent's map of its offset (its pivot kept where it hangs), as high
    /// as its parent if both stand on the stage. Its parent gone, it stays.
    fn hang_now(&mut self, node: &NodeRef) {
        let Some(kept) = self.nodes.get(node).filter(|kept| kept.inherit.pos).cloned() else {
            return;
        };
        let Some(parent) = kept.parent.clone() else {
            return;
        };
        let Some(map) = self.map_of(&parent) else {
            return;
        };
        let pivot = kept.transform.pivot;
        let world = map.transform_point2(kept.offset + pivot) - pivot;
        let owner = world - kept.transform.pos;
        let lift = match &parent {
            NodeRef::Who(by) => self.cast.get(by).map(Actor::shown_height),
            _ => None,
        };
        match node {
            NodeRef::Who(who) => self.with(*who, |actor| {
                (actor.position, actor.way.to, actor.way.left) = (owner, owner, 0.0);
                if let Some(lift) = lift {
                    actor.air.height = lift;
                }
            }),
            NodeRef::Named(name) => match self.words.pictures.iter_mut().find(|picture| picture.name == *name) {
                Some(picture) => picture.at = owner,
                None => _ = self.nodes.insert(node.clone(), Node { transform: Transform { pos: world, ..kept.transform }, ..kept }),
            },
            NodeRef::Halo(rank) => {
                if let Some(halo) = self.light.halos.get_mut(usize::from(*rank)) {
                    halo.on = Cam::At(Point::at(owner));
                }
            }
            NodeRef::Caption | NodeRef::Emitter(_) => {}
        }
    }

    /// A value, worked out, put where it is kept: on its owner if it has it,
    /// else on the node.
    fn put_value(&mut self, node: &NodeRef, kind: NodeKind, prop: PropId, value: Value) -> Result<(), SetError> {
        let number = value.number();
        let point = value.point();
        let flag = value.flag();
        match (node, prop) {
            (NodeRef::Who(who), _) if self.put_on_actor(*who, prop, value)? => return Ok(()),
            (NodeRef::Named(name), PropId::Position | PropId::X | PropId::Y) if kind == NodeKind::Picture => {
                let picture = self.words.pictures.iter_mut().find(|picture| picture.name == *name).ok_or(SetError::NoNode)?;
                match prop {
                    PropId::X => picture.at.x = number.ok_or(SetError::WrongValue)?,
                    PropId::Y => picture.at.y = number.ok_or(SetError::WrongValue)?,
                    _ => picture.at = point.ok_or(SetError::WrongValue)?,
                }
                return Ok(());
            }
            (NodeRef::Halo(rank), PropId::Position | PropId::Radius | PropId::Colour) => {
                let halo = self.light.halos.get_mut(usize::from(*rank)).ok_or(SetError::NoNode)?;
                match prop {
                    PropId::Radius => halo.radius = number.ok_or(SetError::WrongValue)?,
                    PropId::Colour => halo.colour = value.colour().map(|[r, g, b, _]| [r, g, b]).ok_or(SetError::WrongValue)?,
                    _ => halo.on = Cam::At(Point::at(point.ok_or(SetError::WrongValue)?)),
                }
                return Ok(());
            }
            (NodeRef::Emitter(who), PropId::Rate | PropId::Colour) => {
                let emitter = self.light.emitters.iter_mut().find(|emitter| emitter.who == *who).ok_or(SetError::NoNode)?;
                match prop {
                    // (None a second: it gives off no more, waiting for ever.)
                    PropId::Rate => emitter.between = number.filter(|rate| *rate > 0.0).map_or(f32::MAX, |rate| 60.0 / rate),
                    _ => emitter.colour = Some(value.colour().map(|[r, g, b, _]| [r, g, b]).ok_or(SetError::WrongValue)?),
                }
                return Ok(());
            }
            _ => {}
        }
        // (What the node keeps itself.)
        let mut kept = self.kept(node, kind);
        let (transform, looks) = (&mut kept.transform, &mut kept.looks);
        let wrong = SetError::WrongValue;
        match prop {
            PropId::Position => transform.pos = point.ok_or(wrong)?,
            PropId::X => transform.pos.x = number.ok_or(wrong)?,
            PropId::Y => transform.pos.y = number.ok_or(wrong)?,
            PropId::Angle => transform.angle_deg = number.ok_or(wrong)?,
            PropId::Scale => transform.scale = Vec2::splat(number.ok_or(wrong)?),
            PropId::ScaleX => transform.scale.x = number.ok_or(wrong)?,
            PropId::ScaleY => transform.scale.y = number.ok_or(wrong)?,
            PropId::SkewX => transform.skew.x = number.ok_or(wrong)?,
            PropId::SkewY => transform.skew.y = number.ok_or(wrong)?,
            PropId::Pivot => transform.pivot = point.ok_or(wrong)?,
            PropId::FlipX => transform.flip_x = flag.ok_or(wrong)?,
            PropId::FlipY => transform.flip_y = flag.ok_or(wrong)?,
            PropId::Opacity => looks.opacity = number.ok_or(wrong)?,
            PropId::Tint => looks.tint = value.colour().ok_or(wrong)?,
            PropId::Add => looks.add = value.colour().map(|[r, g, b, _]| [r, g, b]).ok_or(wrong)?,
            PropId::Fill => looks.fill = value.colour(),
            PropId::Blend => looks.blend = value.choice().and_then(Blend::named).ok_or(wrong)?,
            PropId::Smooth => looks.smooth = flag.ok_or(wrong)?,
            PropId::Z => kept.z = number,
            PropId::Visible => kept.visible = flag.ok_or(wrong)?,
            PropId::Outline => looks.effects.outline = value.colour(),
            PropId::OutlineSize => looks.effects.outline_width = number.ok_or(wrong)?,
            PropId::Glow => looks.effects.glow = value.colour(),
            PropId::GlowSize => looks.effects.glow_width = number.ok_or(wrong)?,
            PropId::DropShadow => looks.effects.shadow = value.colour(),
            PropId::ShadowOffset => looks.effects.shadow_offset = point.ok_or(wrong)?,
            PropId::Dissolve => looks.effects.dissolve = number.ok_or(wrong)?,
            PropId::Seed => looks.effects.seed = number.ok_or(wrong)?,
            PropId::Wave => looks.effects.wave = number.ok_or(wrong)?,
            PropId::WaveLength => looks.effects.wave_length = number.ok_or(wrong)?,
            PropId::WaveSpeed => looks.effects.wave_speed = number.ok_or(wrong)?,
            PropId::SwapFrom => looks.effects.swap_from = value.colour(),
            PropId::SwapTo => looks.effects.swap_to = value.colour(),
            PropId::Flash => looks.effects.flash = value.colour(),
            PropId::FlashAmount => looks.effects.flash_amount = number.ok_or(wrong)?,
            _ => return Err(SetError::NotOfKind(kind)),
        }
        self.nodes.insert(node.clone(), kept);
        Ok(())
    }

    /// A property a body keeps itself, set on it (false: the node keeps
    /// it). Its scale and flip along both ways are its own, those of one
    /// way the node's, over its own.
    fn put_on_actor(&mut self, who: Who, prop: PropId, value: Value) -> Result<bool, SetError> {
        let kept = self.nodes.get(&NodeRef::Who(who)).map_or(Transform::default(), |kept| kept.transform);
        let actor = self.cast.get_mut(&who).ok_or(SetError::NoNode)?;
        let wrong = SetError::WrongValue;
        let (number, point, flag) = (value.number(), value.point(), value.flag());
        // (Where it is put stops where it was going, if it was not walking.)
        let placed = |actor: &mut Actor, at: Vec2| {
            actor.position = at;
            if actor.way.left <= 0.0 && actor.way.route.is_empty() {
                actor.way.to = at;
            }
        };
        let mut node = None;
        match prop {
            PropId::Position => placed(actor, point.ok_or(wrong)? - kept.pos),
            PropId::X => placed(actor, Vec2::new(number.ok_or(wrong)? - kept.pos.x, actor.position.y)),
            PropId::Y => placed(actor, Vec2::new(actor.position.x, number.ok_or(wrong)? - kept.pos.y)),
            PropId::Scale => {
                (actor.looks.scale, actor.looks.scaling) = (number.ok_or(wrong)?, None);
                node = Some(Transform { scale: Vec2::ONE, ..kept });
            }
            // (One way: the node's share over its own size, that size kept.)
            PropId::ScaleX | PropId::ScaleY => {
                let shown = number.ok_or(wrong)?;
                if actor.looks.scale == 0.0 {
                    actor.looks.scale = 1.0;
                }
                let mut scale = kept.scale;
                match prop {
                    PropId::ScaleX => scale.x = shown / actor.looks.scale,
                    _ => scale.y = shown / actor.looks.scale,
                }
                node = Some(Transform { scale, ..kept });
            }
            PropId::FlipX => {
                actor.flipped = flag.ok_or(wrong)?;
                node = Some(Transform { flip_x: false, ..kept });
            }
            // (Its angle as told: a spin that turned it is over.)
            PropId::Angle => {
                (actor.air.turn, actor.air.spin) = (0.0, actor.air.spin.filter(|spin| !spin.rotate));
                node = Some(Transform { angle_deg: number.ok_or(wrong)?, ..kept });
            }
            PropId::Opacity => (actor.looks.opacity, actor.looks.fading) = (number.ok_or(wrong)?, None),
            PropId::Tint => {
                let [r, g, b, a] = value.colour().ok_or(wrong)?;
                (actor.colour, actor.looks.colouring) = ([r, g, b].map(f32::from), None);
                let mut looks = self.nodes.get(&NodeRef::Who(who)).map_or(Looks::default(), |kept| kept.looks.clone());
                looks.tint = [255, 255, 255, a];
                let mut kept = self.kept(&NodeRef::Who(who), if who.is_thing() { NodeKind::Thing } else { NodeKind::Body });
                kept.looks = looks;
                self.nodes.insert(NodeRef::Who(who), kept);
                return Ok(true);
            }
            PropId::Visible => {
                actor.attributes = match flag.ok_or(wrong)? {
                    true => actor.attributes & !attribute::HIDDEN,
                    false => actor.attributes | attribute::HIDDEN,
                }
            }
            PropId::Height => (actor.air.height, actor.air.rise) = (number.ok_or(wrong)?, None),
            PropId::Sink => actor.sunk = number.ok_or(wrong)?,
            PropId::Depth => actor.looks.depth = number.ok_or(wrong)?,
            PropId::Slowmo => actor.looks.time = number.ok_or(wrong)?,
            PropId::AnimSpeed => actor.looks.speed = number.ok_or(wrong)?,
            PropId::Shadow => actor.shadowless = !flag.ok_or(wrong)?,
            PropId::Facing => actor.direction = value.choice().and_then(Direction::of).ok_or(wrong)?.number(),
            _ => return Ok(false),
        }
        if let Some(transform) = node {
            let mut kept = self.kept(&NodeRef::Who(who), if who.is_thing() { NodeKind::Thing } else { NodeKind::Body });
            kept.transform = transform;
            self.nodes.insert(NodeRef::Who(who), kept);
        }
        Ok(true)
    }

    /// Plays a node's beat (a track's: `stage::tracks`); gives back none.
    pub(super) fn direct_node(&mut self, beat: NodeBeat) -> Option<NodeBeat> {
        match beat {
            // (What cannot be set is told by the checks, before it plays.)
            NodeBeat::Set { node, prop, value } => _ = self.set(&node, prop, &value),
            NodeBeat::Make { name, sort, at } => {
                let node = NodeRef::Named(name.clone());
                self.words.pictures.retain(|picture| picture.name != name);
                match sort {
                    Sort::Group => {
                        let mut made = Node::new(name, NodeKind::Group);
                        made.transform.pos = at.vec2();
                        self.nodes.insert(node, made);
                    }
                    Sort::Picture(file) => {
                        self.words.pictures.push(PlacedPicture { name: name.clone(), file, at: at.vec2() });
                        self.nodes.insert(node, Node::new(name, NodeKind::Picture));
                    }
                }
            }
            NodeBeat::Unmake(name) => {
                self.words.pictures.retain(|picture| picture.name != name);
                self.nodes.remove(&NodeRef::Named(name));
            }
            // (What cannot hang is told by the checks, before it plays.)
            NodeBeat::Attach { node, to, offset, inherit } => _ = self.attach(&node, &to, offset, inherit),
            NodeBeat::Detach(node) => self.detach(&node),
            track @ (NodeBeat::Animate { .. } | NodeBeat::Path { .. } | NodeBeat::Stop { .. }) => self.direct_track(track),
        }
        None
    }
}
