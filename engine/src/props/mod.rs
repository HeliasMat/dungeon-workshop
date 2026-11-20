//! The one registry of what a scene shows and what can be set on it, the
//! single source of truth: every kind of node ([`NodeKind`]) and every
//! property ([`Prop`]: its name, its type, unit,
//! default, advised range, whether it can be animated, the kinds it is
//! of). The film words (`set`, later `anim`), the checks, the docs'
//! tables and the machine outputs read it; nothing keeps a list of its own.
//!
//! - `value`: the one value grammar every word shares (durations, pixels,
//!   angles, factors, percentages, colours, points, relative values).
//!
//! A property is added here, once: its line in [`PROPS`], its [`PropId`],
//! and where the stage keeps it (`Stage::get`, `Stage::put`,
//! `engine/src/stage/node.rs`).

use crate::problem::codes;
use diagnostics::text::{distance, nearest};

#[cfg(test)]
mod tests;
mod value;

pub use value::*;

/// A kind of node: what a scene shows, each with its properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    /// Someone of the cast: a creature, a character.
    Body,
    /// A thing: an item, a prop, a picture of a film's own.
    Thing,
    /// A picture from a file put on the scene (`node n picture f.png`).
    Picture,
    /// The few words over the scene (`caption`).
    Caption,
    /// A soft light (`halo`).
    Halo,
    /// What gives off particles, on someone (`emit`).
    Emitter,
    /// A node of nothing but its place: something to hang others on
    /// (`node n group`).
    Group,
}

impl NodeKind {
    pub const ALL: [NodeKind; 7] = [NodeKind::Body, NodeKind::Thing, NodeKind::Picture, NodeKind::Caption, NodeKind::Halo, NodeKind::Emitter, NodeKind::Group];

    /// Its name, as films write it.
    pub fn name(self) -> &'static str {
        match self {
            NodeKind::Body => "body",
            NodeKind::Thing => "thing",
            NodeKind::Picture => "picture",
            NodeKind::Caption => "caption",
            NodeKind::Halo => "halo",
            NodeKind::Emitter => "emitter",
            NodeKind::Group => "group",
        }
    }

    /// A kind by its name.
    pub fn named(name: &str) -> Option<NodeKind> {
        NodeKind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    const fn bit(self) -> u16 {
        1 << self as u16
    }
}

/// The kinds of node a property is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kinds(u16);

impl Kinds {
    /// Every kind.
    pub const ALL: Kinds = Kinds::of(&NodeKind::ALL);
    /// Bodies and things: what stands on the stage.
    pub const STANDING: Kinds = Kinds::of(&[NodeKind::Body, NodeKind::Thing]);
    /// What is drawn from a picture: bodies, things, pictures (their
    /// effects, their smoothing).
    pub const DRAWN: Kinds = Kinds::of(&[NodeKind::Body, NodeKind::Thing, NodeKind::Picture]);

    pub const fn of(kinds: &[NodeKind]) -> Kinds {
        let mut bits = 0;
        let mut at = 0;
        while at < kinds.len() {
            bits |= kinds[at].bit();
            at += 1;
        }
        Kinds(bits)
    }

    pub fn has(self, kind: NodeKind) -> bool {
        self.0 & kind.bit() != 0
    }

    /// The kinds, in order.
    pub fn iter(self) -> impl Iterator<Item = NodeKind> {
        NodeKind::ALL.into_iter().filter(move |kind| self.has(*kind))
    }
}

/// What a property's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Number,
    Bool,
    /// Two numbers, `x y` (or a node's point, `@a`).
    Point,
    Colour,
    /// One of these words.
    Choice(&'static [&'static str]),
}

impl Type {
    /// What it is, as a problem says it.
    pub fn said(self) -> String {
        match self {
            Type::Number => "a number".to_string(),
            Type::Bool => "on or off".to_string(),
            Type::Point => "a point (x y, @a, @a+10,-4)".to_string(),
            Type::Colour => "a colour (#ff8800, #ff880080, red, rgb(255,136,0))".to_string(),
            Type::Choice(words) => format!("one of: {}", words.join(", ")),
        }
    }

    /// Its name, for the tables.
    pub fn name(self) -> &'static str {
        match self {
            Type::Number => "number",
            Type::Bool => "yes/no",
            Type::Point => "point",
            Type::Colour => "colour",
            Type::Choice(_) => "choice",
        }
    }
}

/// What a property's numbers count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    None,
    /// Pixels of the scene (`12`, `12px`).
    Px,
    /// Degrees (`45`, `45deg`, `0.5turn`).
    Deg,
    /// A factor (`1.5`, `150%`).
    Factor,
    /// A share from 0 to 1 (`0.5`, `50%`).
    Ratio,
    /// So many a second (`5`, `5/s`).
    PerSecond,
    /// Frames, 60 a second (`2s`, `30f`).
    Frames,
}

impl Unit {
    /// How a value of it is written, for the tables.
    pub fn written(self) -> &'static str {
        match self {
            Unit::None => "",
            Unit::Px => "px",
            Unit::Deg => "deg, turn",
            Unit::Factor => "x, %",
            Unit::Ratio => "0..1, %",
            Unit::PerSecond => "/s",
            Unit::Frames => "s, f",
        }
    }
}

/// Every property, by what it is: the stage keeps each (see `Stage::get`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PropId {
    Position,
    X,
    Y,
    Angle,
    Scale,
    ScaleX,
    ScaleY,
    SkewX,
    SkewY,
    Pivot,
    FlipX,
    FlipY,
    Opacity,
    Tint,
    Add,
    Fill,
    Blend,
    Smooth,
    Z,
    Visible,
    Height,
    Sink,
    Depth,
    Slowmo,
    AnimSpeed,
    Shadow,
    Facing,
    Radius,
    Colour,
    Rate,
    Outline,
    OutlineSize,
    Glow,
    GlowSize,
    DropShadow,
    ShadowOffset,
    Dissolve,
    Seed,
    Wave,
    WaveLength,
    WaveSpeed,
    SwapFrom,
    SwapTo,
    Flash,
    FlashAmount,
}

impl PropId {
    pub const ALL: [PropId; 45] = [
        PropId::Position,
        PropId::X,
        PropId::Y,
        PropId::Angle,
        PropId::Scale,
        PropId::ScaleX,
        PropId::ScaleY,
        PropId::SkewX,
        PropId::SkewY,
        PropId::Pivot,
        PropId::FlipX,
        PropId::FlipY,
        PropId::Opacity,
        PropId::Tint,
        PropId::Add,
        PropId::Fill,
        PropId::Blend,
        PropId::Smooth,
        PropId::Z,
        PropId::Visible,
        PropId::Height,
        PropId::Sink,
        PropId::Depth,
        PropId::Slowmo,
        PropId::AnimSpeed,
        PropId::Shadow,
        PropId::Facing,
        PropId::Radius,
        PropId::Colour,
        PropId::Rate,
        PropId::Outline,
        PropId::OutlineSize,
        PropId::Glow,
        PropId::GlowSize,
        PropId::DropShadow,
        PropId::ShadowOffset,
        PropId::Dissolve,
        PropId::Seed,
        PropId::Wave,
        PropId::WaveLength,
        PropId::WaveSpeed,
        PropId::SwapFrom,
        PropId::SwapTo,
        PropId::Flash,
        PropId::FlashAmount,
    ];

    /// Its line in the registry.
    pub fn prop(self) -> &'static Prop {
        PROPS.iter().find(|prop| prop.id == self).expect("every property is in the registry")
    }
}

/// A property: how films name it, what its value is, what it is by
/// default, what is advised, what is impossible, the kinds it is of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub id: PropId,
    /// Its name, as films write it.
    pub name: &'static str,
    pub ty: Type,
    pub unit: Unit,
    /// Its value on a node just made (`Value::Nothing`: none, as for a
    /// fill, or as the kind has it, as for `z`).
    pub default: Value,
    /// The values that make sense: past them it plays as asked, with a
    /// remark (`R-OUT-OF-RANGE`).
    pub advised: Option<(f32, f32)>,
    /// Below it a value is impossible (a negative size), an error.
    pub floor: Option<f32>,
    /// Whether it can change in time (a track, `anim`).
    pub animatable: bool,
    pub kinds: Kinds,
    pub doc: &'static str,
}

/// The ways to face, as a property's choices.
const WAYS: &[&str] = &["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"];

/// A property that every kind of node has, of numbers.
const fn number(id: PropId, name: &'static str, unit: Unit, default: f32, doc: &'static str) -> Prop {
    Prop { id, name, ty: Type::Number, unit, default: Value::Number(default), advised: None, floor: None, animatable: true, kinds: Kinds::ALL, doc }
}

const fn switch(id: PropId, name: &'static str, default: bool, doc: &'static str) -> Prop {
    Prop { id, name, ty: Type::Bool, unit: Unit::None, default: Value::Bool(default), advised: None, floor: None, animatable: false, kinds: Kinds::ALL, doc }
}

/// A colour of an effect, none by default (none: no such effect).
const fn effect_colour(id: PropId, name: &'static str, doc: &'static str) -> Prop {
    Prop { id, name, ty: Type::Colour, unit: Unit::None, default: Value::Nothing, advised: None, floor: None, animatable: true, kinds: Kinds::DRAWN, doc }
}

impl Prop {
    const fn advised(self, least: f32, most: f32) -> Prop {
        Prop { advised: Some((least, most)), ..self }
    }

    const fn floor(self, floor: f32) -> Prop {
        Prop { floor: Some(floor), ..self }
    }

    const fn of(self, kinds: Kinds) -> Prop {
        Prop { kinds, ..self }
    }

    const fn still(self) -> Prop {
        Prop { animatable: false, ..self }
    }
}

/// The registry: every property, once.
pub static PROPS: &[Prop] = &[
    Prop {
        id: PropId::Position,
        name: "position",
        ty: Type::Point,
        unit: Unit::Px,
        default: Value::Point(glam::Vec2::ZERO),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "where it is (a body: its feet), in pixels of the scene",
    },
    number(PropId::X, "x", Unit::Px, 0.0, "its position, from left to right"),
    number(PropId::Y, "y", Unit::Px, 0.0, "its position, from top to bottom"),
    number(PropId::Angle, "angle", Unit::Deg, 0.0, "turned by so many degrees, around its pivot (clockwise)"),
    number(PropId::Scale, "scale", Unit::Factor, 1.0, "its size (1: its own), in width and height").floor(0.0).advised(0.05, 8.0),
    number(PropId::ScaleX, "scale_x", Unit::Factor, 1.0, "its size in width").floor(0.0).advised(0.05, 8.0),
    number(PropId::ScaleY, "scale_y", Unit::Factor, 1.0, "its size in height").floor(0.0).advised(0.05, 8.0),
    number(PropId::SkewX, "skew_x", Unit::Deg, 0.0, "slanted by so many degrees, sideways").advised(-80.0, 80.0),
    number(PropId::SkewY, "skew_y", Unit::Deg, 0.0, "slanted by so many degrees, up and down").advised(-80.0, 80.0),
    Prop {
        id: PropId::Pivot,
        name: "pivot",
        ty: Type::Point,
        unit: Unit::Px,
        default: Value::Point(glam::Vec2::ZERO),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "the point it turns and grows around, from its position",
    },
    switch(PropId::FlipX, "flip_x", false, "flipped from left to right"),
    switch(PropId::FlipY, "flip_y", false, "flipped upside down"),
    number(PropId::Opacity, "opacity", Unit::Ratio, 1.0, "how much of it shows (1: all, 0: nothing)").floor(0.0).advised(0.0, 1.0),
    Prop {
        id: PropId::Tint,
        name: "tint",
        ty: Type::Colour,
        unit: Unit::None,
        default: Value::Colour([255, 255, 255, 255]),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "the colour multiplying its own (white: its own, black: a silhouette)",
    },
    Prop {
        id: PropId::Add,
        name: "add",
        ty: Type::Colour,
        unit: Unit::None,
        default: Value::Colour([0, 0, 0, 255]),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "the colour added to its own (black: nothing), a flash",
    },
    Prop {
        id: PropId::Fill,
        name: "fill",
        ty: Type::Colour,
        unit: Unit::None,
        default: Value::Nothing,
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "painted all in one colour, its shape kept (none: no)",
    },
    Prop {
        id: PropId::Blend,
        name: "blend",
        ty: Type::Choice(Blend::NAMES),
        unit: Unit::None,
        default: Value::Choice("normal"),
        advised: None,
        floor: None,
        animatable: false,
        kinds: Kinds::ALL,
        doc: "how it blends with what is under it",
    },
    switch(PropId::Smooth, "smooth", false, "turned or grown, smoothed (bilinear) rather than in sharp pixels").of(Kinds::DRAWN),
    Prop {
        id: PropId::Z,
        name: "z",
        ty: Type::Number,
        unit: Unit::None,
        default: Value::Nothing,
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::ALL,
        doc: "its rank, in front of those of smaller z (none: as its kind, bodies by their feet; the “z ranks” table)",
    },
    switch(PropId::Visible, "visible", true, "whether it shows"),
    number(PropId::Height, "height", Unit::Px, 0.0, "so many pixels above the ground").of(Kinds::STANDING).advised(-64.0, 400.0),
    number(PropId::Sink, "sink", Unit::Px, 0.0, "sunk so deep in water (or sand), the part below hidden").of(Kinds::STANDING).floor(0.0).advised(0.0, 64.0),
    number(PropId::Depth, "depth", Unit::Px, 0.0, "drawn as if it were so much lower, among the others").of(Kinds::STANDING),
    number(PropId::Slowmo, "slowmo", Unit::Factor, 1.0, "its own time (1: the scene's, 0.3: slowed down)").of(Kinds::STANDING).floor(0.0).advised(0.05, 8.0),
    number(PropId::AnimSpeed, "animspeed", Unit::Factor, 1.0, "the speed of its frames (1: their own)").of(Kinds::STANDING).floor(0.0).advised(0.05, 8.0),
    switch(PropId::Shadow, "shadow", true, "whether it has its shadow").of(Kinds::STANDING),
    Prop {
        id: PropId::Facing,
        name: "facing",
        ty: Type::Choice(WAYS),
        unit: Unit::None,
        default: Value::Choice("down"),
        advised: None,
        floor: None,
        animatable: false,
        kinds: Kinds::STANDING,
        doc: "where it looks, one of the eight directions",
    },
    number(PropId::Radius, "radius", Unit::Px, 40.0, "its width, from the middle to the edge").of(Kinds::of(&[NodeKind::Halo])).floor(0.0).advised(1.0, 400.0),
    Prop {
        id: PropId::Colour,
        name: "colour",
        ty: Type::Colour,
        unit: Unit::None,
        default: Value::Colour([255, 255, 255, 255]),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::of(&[NodeKind::Halo, NodeKind::Emitter]),
        doc: "its colour (a body: tint)",
    },
    number(PropId::Rate, "rate", Unit::PerSecond, 4.0, "so many particles a second").of(Kinds::of(&[NodeKind::Emitter])).floor(0.0).advised(0.1, 60.0).still(),
    effect_colour(PropId::Outline, "outline", "an outline of this colour around its shape (none: no)"),
    number(PropId::OutlineSize, "outline_size", Unit::Px, 1.0, "the outline's width, in pixels of the scene").of(Kinds::DRAWN).floor(0.0).advised(0.5, 8.0),
    effect_colour(PropId::Glow, "glow", "a glow of this colour around it, in three sharp bands (none: no)"),
    number(PropId::GlowSize, "glow_size", Unit::Px, 3.0, "the glow's width, in pixels of the scene").of(Kinds::DRAWN).floor(0.0).advised(1.0, 16.0),
    effect_colour(PropId::DropShadow, "drop_shadow", "a shadow of its shape, of this colour, offset (none: no)"),
    Prop {
        id: PropId::ShadowOffset,
        name: "shadow_offset",
        ty: Type::Point,
        unit: Unit::Px,
        default: Value::Point(glam::Vec2::new(2.0, 2.0)),
        advised: None,
        floor: None,
        animatable: true,
        kinds: Kinds::DRAWN,
        doc: "how far the drop shadow is offset, in pixels of the scene",
    },
    number(PropId::Dissolve, "dissolve", Unit::Ratio, 0.0, "the share of its pixels gone (0: none, 1: all), drawn by its seed")
        .of(Kinds::DRAWN)
        .floor(0.0)
        .advised(0.0, 1.0),
    number(PropId::Seed, "seed", Unit::None, 0.0, "the seed of its dissolve: same seed, same pixels").of(Kinds::DRAWN).still(),
    number(PropId::Wave, "wave", Unit::Px, 0.0, "its lines wave by so many pixels of its picture, sideways (0: no)").of(Kinds::DRAWN).advised(-16.0, 16.0),
    number(PropId::WaveLength, "wave_length", Unit::Px, 16.0, "one wave every so many lines of its picture").of(Kinds::DRAWN).floor(0.0).advised(1.0, 256.0),
    number(PropId::WaveSpeed, "wave_speed", Unit::PerSecond, 1.0, "so many waves a second (0: still)").of(Kinds::DRAWN).advised(-20.0, 20.0),
    effect_colour(PropId::SwapFrom, "swap_from", "a colour of its picture, changed into swap_to (none: no)"),
    effect_colour(PropId::SwapTo, "swap_to", "the colour swap_from takes"),
    effect_colour(PropId::Flash, "flash", "it turns to this colour, by flash_amount (none: no); a silhouette: fill"),
    number(PropId::FlashAmount, "flash_amount", Unit::Ratio, 1.0, "how far it turns to the flash's colour (1: all)")
        .of(Kinds::DRAWN)
        .floor(0.0)
        .advised(0.0, 1.0),
];

/// A property by its name.
pub fn find(name: &str) -> Option<&'static Prop> {
    PROPS.iter().find(|prop| prop.name == name)
}

/// The properties of a kind of node.
pub fn of_kind(kind: NodeKind) -> impl Iterator<Item = &'static Prop> {
    PROPS.iter().filter(move |prop| prop.kinds.has(kind))
}

/// The names nearest to a name no property has, nearest first: at most
/// three, and only near ones.
pub fn closest(name: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = PROPS.iter().map(|prop| prop.name).collect();
    names.sort_unstable();
    let near = |own: &str| Some(distance(name, own)).filter(|far| *far <= (own.len().max(name.len()) / 2).max(1)).map(|far| -(far as f64));
    nearest(names, 3, near)
}

/// What is wrong with a property's name: the code, the message, the
/// nearest names.
pub fn unknown(name: &str) -> ValueError {
    let near = closest(name);
    let message = match near.as_slice() {
        [] => format!("unknown property “{name}” (the “Properties” table of film/FILMS.md)"),
        near => format!("unknown property “{name}”. Near: {}", near.join(", ")),
    };
    ValueError { code: codes::PROP_UNKNOWN, message, fix: near.first().map(|near| near.to_string()) }
}

impl Prop {
    /// Whether a kind of node has it.
    pub fn fits(&self, kind: NodeKind) -> bool {
        self.kinds.has(kind)
    }

    /// What a value of it is, checked: an error if it is impossible (not a
    /// number, under its floor), a remark if past what is advised; none if
    /// it is fine. A value written relative is known only as it plays.
    pub fn check(&self, value: &Value) -> Result<Option<ValueError>, ValueError> {
        let numbers: Vec<f32> = match value {
            Value::Number(number) => vec![*number],
            Value::Point(at) => vec![at.x, at.y],
            _ => return Ok(None),
        };
        if numbers.iter().any(|number| !number.is_finite()) {
            let message = format!("“{value}” is not a finite number for {}", self.name);
            return Err(ValueError { code: codes::VALUE_IMPOSSIBLE, message, fix: Some(self.default.to_string()) });
        }
        if let (Some(floor), Value::Number(number)) = (self.floor, value)
            && *number < floor
        {
            let message = format!("{} cannot be {number}: {floor} at least", self.name);
            return Err(ValueError { code: codes::VALUE_IMPOSSIBLE, message, fix: Some(floor.to_string()) });
        }
        if let (Some((least, most)), Value::Number(number)) = (self.advised, value)
            && !(least..=most).contains(number)
        {
            let message = format!("{} {number}: outside what is advised ({least} to {most}), played as it is", self.name);
            return Ok(Some(ValueError { code: codes::OUT_OF_RANGE, message, fix: None }));
        }
        Ok(None)
    }
}

/// The registry as a table for the docs (`film/FILMS.md` keeps it, checked
/// by `film/src/tests/docs.rs`): each property, its type, unit,
/// default, advised range, whether it animates, its kinds, what it is.
pub fn table() -> String {
    let mut out = String::from("| Property | Type | Unit | Default | Advised | Animates | Kinds | What it is |\n|---|---|---|---|---|---|---|---|\n");
    for prop in PROPS {
        let ty = match prop.ty {
            Type::Choice(words) => format!("choice: {}", words.join(", ")),
            other => other.name().to_string(),
        };
        let advised = match (prop.advised, prop.floor) {
            (Some((least, most)), _) => format!("{least} to {most}"),
            (None, Some(floor)) => format!("{floor} at least"),
            (None, None) => String::new(),
        };
        let kinds: Vec<&str> = prop.kinds.iter().map(NodeKind::name).collect();
        let kinds = if prop.kinds == Kinds::ALL { "all".to_string() } else { kinds.join(", ") };
        out.push_str(&format!(
            "| `{}` | {ty} | {} | `{}` | {advised} | {} | {kinds} | {} |\n",
            prop.name,
            prop.unit.written(),
            prop.default,
            if prop.animatable { "yes" } else { "no" },
            prop.doc
        ));
    }
    out
}
