//! The one value grammar, shared by every word that takes a value:
//!
//! - durations: `2s`, `1.5s`, `250ms`, `30f` (a bare number: frames);
//! - distances: `12`, `12px`; angles: `45`, `45deg`, `0.5turn`;
//! - factors: `1.5`, `150%`; shares: `0.5`, `50%`; so many a second: `5/s`;
//! - colours: `#f80`, `#ff8800`, `#ff880080`, a name (`red`),
//!   `rgb(255,136,0)`, `rgba(255,136,0,0.5)`;
//! - points: `x y`, `x,y`, a node's own `@a`, beside it `@a+10,-4`;
//! - relative: `+30`, `+-30`, `+10,-4` (added to what it is), `x1.5`
//!   (multiplied); `default`: back to the registry's default.
//!
//! A value is read for a property ([`read`]): its type and unit say which
//! of these it may be. What is read is a [`Given`]; a value known as it
//! plays (relative, a node's point) is made a [`Value`] by the stage.

use std::fmt;

use glam::Vec2;

use super::{Prop, Type, Unit};
use crate::base::{Frames, colour_named};
use crate::problem::codes;

/// A colour: red, green, blue, and how much of it shows, of 255.
pub type Rgba = [u8; 4];

/// A property's value, as the stage keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Number(f32),
    Bool(bool),
    Point(Vec2),
    Colour(Rgba),
    /// One of its type's words.
    Choice(&'static str),
    /// None: no fill, `z` as its kind has it.
    Nothing,
}

impl Value {
    pub fn number(&self) -> Option<f32> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }

    pub fn point(&self) -> Option<Vec2> {
        match self {
            Value::Point(at) => Some(*at),
            _ => None,
        }
    }

    pub fn colour(&self) -> Option<Rgba> {
        match self {
            Value::Colour(colour) => Some(*colour),
            _ => None,
        }
    }

    pub fn flag(&self) -> Option<bool> {
        match self {
            Value::Bool(on) => Some(*on),
            _ => None,
        }
    }

    pub fn choice(&self) -> Option<&'static str> {
        match self {
            Value::Choice(word) => Some(word),
            _ => None,
        }
    }

    /// Itself with that added (numbers, points; colours part by part, up to
    /// 255); none for what cannot be added.
    pub fn plus(self, by: Value) -> Option<Value> {
        Some(match (self, by) {
            (Value::Number(number), Value::Number(by)) => Value::Number(number + by),
            (Value::Point(at), Value::Point(by)) => Value::Point(at + by),
            (Value::Colour(colour), Value::Colour(by)) => Value::Colour(std::array::from_fn(|part| colour[part].saturating_add(by[part]))),
            _ => return None,
        })
    }

    /// Itself so many times over (numbers, points).
    pub fn times(self, by: f32) -> Option<Value> {
        Some(match self {
            Value::Number(number) => Value::Number(number * by),
            Value::Point(at) => Value::Point(at * by),
            _ => return None,
        })
    }
}

/// As the grammar reads it back: a point in two words.
impl fmt::Display for Value {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(number) => write!(out, "{number}"),
            Value::Bool(on) => write!(out, "{}", if *on { "on" } else { "off" }),
            Value::Point(at) => write!(out, "{} {}", at.x, at.y),
            Value::Colour([r, g, b, 255]) => write!(out, "#{r:02x}{g:02x}{b:02x}"),
            Value::Colour([r, g, b, a]) => write!(out, "#{r:02x}{g:02x}{b:02x}{a:02x}"),
            Value::Choice(word) => write!(out, "{word}"),
            Value::Nothing => write!(out, "none"),
        }
    }
}

/// A value as written for a property: as it is, added to what it is,
/// multiplied, a node's point (`N`: how the node is named), or back to
/// the default.
#[derive(Clone, Debug, PartialEq)]
pub enum Given<N> {
    Is(Value),
    Plus(Value),
    Times(f32),
    At { node: N, by: Vec2 },
    Default,
}

impl<N> Given<N> {
    /// The same, its node named otherwise; none if it cannot be.
    pub fn map<M>(self, name: impl FnOnce(N) -> Option<M>) -> Option<Given<M>> {
        Some(match self {
            Given::Is(value) => Given::Is(value),
            Given::Plus(value) => Given::Plus(value),
            Given::Times(by) => Given::Times(by),
            Given::At { node, by } => Given::At { node: name(node)?, by },
            Given::Default => Given::Default,
        })
    }

    /// The value as written, if it is known before it plays.
    pub fn value(&self) -> Option<&Value> {
        match self {
            Given::Is(value) => Some(value),
            _ => None,
        }
    }
}

/// As [`read`] reads it back.
impl<N: fmt::Display> fmt::Display for Given<N> {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Given::Is(value) => write!(out, "{value}"),
            Given::Plus(Value::Point(by)) => write!(out, "+{},{}", by.x, by.y),
            Given::Plus(value) => write!(out, "+{value}"),
            Given::Times(by) => write!(out, "x{by}"),
            Given::At { node, by } if *by == Vec2::ZERO => write!(out, "@{node}"),
            Given::At { node, by } => write!(out, "@{node}{:+},{}", by.x, by.y),
            Given::Default => write!(out, "default"),
        }
    }
}

/// What is wrong with a value or a property's name: its stable code (see
/// `problem::codes`), what is said, and what to write instead if known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueError {
    pub code: &'static str,
    pub message: String,
    pub fix: Option<String>,
}

impl fmt::Display for ValueError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.message)
    }
}

/// The value of a property from the first words given: what is given, and
/// how many words it took (a point `x y`: two; `rgb(255, 0, 0)`: as many as
/// it spans).
pub fn read(prop: &Prop, words: &[String]) -> Result<(Given<String>, usize), ValueError> {
    let Some(first) = words.first().map(String::as_str) else {
        let message = format!("the value of {} is missing ({})", prop.name, prop.ty.said());
        return Err(ValueError { code: codes::VALUE_MISSING, message, fix: Some(prop.default.to_string()) });
    };
    let wrong = || {
        let message = format!("“{first}” is not {} for {}", prop.ty.said(), prop.name);
        ValueError { code: codes::VALUE_TYPE, message, fix: Some(prop.default.to_string()) }
    };
    if first == "default" {
        return Ok((Given::Default, 1));
    }
    if matches!(first, "none" | "auto") && prop.default == Value::Nothing {
        return Ok((Given::Is(Value::Nothing), 1));
    }
    let one = |given| Ok((given, 1));
    match prop.ty {
        Type::Point => {
            if let Some(rest) = first.strip_prefix('@') {
                let (node, by) = node_point(rest).ok_or_else(wrong)?;
                return one(Given::At { node, by });
            }
            if let Some(rest) = first.strip_prefix('+') {
                return one(Given::Plus(Value::Point(pair(rest, prop.unit).ok_or_else(wrong)?)));
            }
            if first.contains(',') {
                return one(Given::Is(Value::Point(pair(first, prop.unit).ok_or_else(wrong)?)));
            }
            let x = number_in(first, prop.unit).ok_or_else(wrong)?;
            let y = words.get(1).and_then(|y| number_in(y, prop.unit)).ok_or_else(|| {
                let message = format!("“{first}”: a point has two numbers, x y (or @a) for {}", prop.name);
                ValueError { code: codes::VALUE_TYPE, message, fix: Some(format!("{first} 0")) }
            })?;
            Ok((Given::Is(Value::Point(Vec2::new(x, y))), 2))
        }
        Type::Number => {
            if let Some(rest) = first.strip_prefix('+') {
                return one(Given::Plus(Value::Number(number_in(rest, prop.unit).ok_or_else(wrong)?)));
            }
            if let Some(by) = first.strip_prefix(['x', '×']).and_then(|by| by.parse::<f32>().ok()) {
                return one(Given::Times(by));
            }
            one(Given::Is(Value::Number(number_in(first, prop.unit).ok_or_else(wrong)?)))
        }
        Type::Bool => one(Given::Is(Value::Bool(boolean(first).ok_or_else(wrong)?))),
        Type::Colour => {
            let (colour, used) = colour(words).ok_or_else(wrong)?;
            Ok((Given::Is(Value::Colour(colour)), used))
        }
        Type::Choice(choices) => one(Given::Is(Value::Choice(choices.iter().find(|choice| **choice == first).ok_or_else(wrong)?))),
    }
}

/// A number of that unit as written (`12px`, `45deg`, `0.5turn`, `50%`,
/// `5/s`, `2s`): none if it is not one, or of another unit.
pub fn number_in(text: &str, unit: Unit) -> Option<f32> {
    let bare = |text: &str| text.parse::<f32>().ok();
    match unit {
        Unit::None => bare(text),
        Unit::Px => bare(text.strip_suffix("px").unwrap_or(text)),
        Unit::Deg => match text.strip_suffix("turns").or_else(|| text.strip_suffix("turn")) {
            Some(turns) => bare(turns).map(|turns| turns * 360.0),
            None => bare(text.strip_suffix("deg").or_else(|| text.strip_suffix('°')).unwrap_or(text)),
        },
        Unit::Factor | Unit::Ratio => match text.strip_suffix('%') {
            Some(share) => bare(share).map(|share| share / 100.0),
            None => bare(text),
        },
        Unit::PerSecond => bare(text.strip_suffix("/s").unwrap_or(text)),
        Unit::Frames => duration(text).map(Frames::get),
    }
}

/// A time as written: `2s`, `1.5s`, `250ms`, `30f`; a bare number is frames.
pub fn duration(text: &str) -> Option<Frames> {
    let bare = |text: &str| text.parse::<f32>().ok();
    let frames = if let Some(ms) = text.strip_suffix("ms") {
        bare(ms)? * 60.0 / 1000.0
    } else if let Some(seconds) = text.strip_suffix('s') {
        bare(seconds)? * 60.0
    } else if let Some(frames) = text.strip_suffix('f') {
        bare(frames)?
    } else {
        bare(text)?
    };
    Some(Frames(frames))
}

/// On or off as written.
pub fn boolean(text: &str) -> Option<bool> {
    match text {
        "on" | "yes" | "true" | "1" => Some(true),
        "off" | "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

/// A colour from the first words (`#ff8800`, `red`, `rgb(255, 136, 0)`):
/// it, and how many words it took.
pub fn colour(words: &[String]) -> Option<(Rgba, usize)> {
    let first = words.first()?;
    if let Some(hex) = first.strip_prefix('#') {
        return Some((hex_colour(hex)?, 1));
    }
    if first.starts_with("rgb(") || first.starts_with("rgba(") {
        let used = words.iter().position(|word| word.ends_with(')'))? + 1;
        let text: String = words[..used].concat();
        let inner = text.split_once('(')?.1.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        let channel = |part: &str| part.parse::<f32>().ok().filter(|part| (0.0..=255.0).contains(part)).map(|part| part.round() as u8);
        let alpha = |part: &str| match part.strip_suffix('%') {
            Some(share) => share.parse::<f32>().ok().filter(|share| (0.0..=100.0).contains(share)).map(|share| (share * 2.55).round() as u8),
            None => part.parse::<f32>().ok().filter(|share| (0.0..=1.0).contains(share)).map(|share| (share * 255.0).round() as u8),
        };
        return match parts.as_slice() {
            [r, g, b] => Some(([channel(r)?, channel(g)?, channel(b)?, 255], used)),
            [r, g, b, a] => Some(([channel(r)?, channel(g)?, channel(b)?, alpha(a)?], used)),
            _ => None,
        };
    }
    let [r, g, b] = colour_named(first)?.map(|part| part as u8);
    Some(([r, g, b, 255], 1))
}

/// A colour of 3, 4, 6 or 8 hex digits.
fn hex_colour(hex: &str) -> Option<Rgba> {
    let digit = |at: usize| u8::from_str_radix(hex.get(at..at + 1)?, 16).ok();
    let byte = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    match hex.len() {
        3 | 4 => {
            let short = |at: usize| digit(at).map(|digit| digit * 17);
            Some([short(0)?, short(1)?, short(2)?, if hex.len() == 4 { short(3)? } else { 255 }])
        }
        6 | 8 => Some([byte(0)?, byte(2)?, byte(4)?, if hex.len() == 8 { byte(6)? } else { 255 }]),
        _ => None,
    }
}

/// Two numbers of a unit, `x,y`.
fn pair(text: &str, unit: Unit) -> Option<Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(Vec2::new(number_in(x, unit)?, number_in(y, unit)?))
}

/// A node and how far from its point (`a`, `a+10,-4`, `a-3,2`).
fn node_point(text: &str) -> Option<(String, Vec2)> {
    match text.char_indices().skip(1).find(|(_, letter)| matches!(letter, '+' | '-')) {
        Some((at, _)) => Some((text[..at].to_string(), pair(&text[at..], Unit::Px)?)),
        None => (!text.is_empty()).then(|| (text.to_string(), Vec2::ZERO)),
    }
}

/// How a node shows over what is under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Blend {
    #[default]
    Normal,
    Add,
    Multiply,
    Screen,
    Overlay,
    Lighten,
    Darken,
    Difference,
}

impl Blend {
    /// Their names, in order, as films write them.
    pub const NAMES: &'static [&'static str] = &["normal", "add", "multiply", "screen", "overlay", "lighten", "darken", "difference"];
    const ALL: [Blend; 8] = [Blend::Normal, Blend::Add, Blend::Multiply, Blend::Screen, Blend::Overlay, Blend::Lighten, Blend::Darken, Blend::Difference];

    pub fn name(self) -> &'static str {
        Blend::NAMES[self as usize]
    }

    pub fn named(name: &str) -> Option<Blend> {
        Blend::ALL.into_iter().find(|blend| blend.name() == name)
    }
}
