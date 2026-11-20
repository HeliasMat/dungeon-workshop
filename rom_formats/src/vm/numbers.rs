//! What the parameters of operations mean.

use super::{Act, Place};

pub(super) fn signed(parameter: u16) -> i32 {
    parameter as i16 as i32
}

/// A number of the staging operations: 15 bits, signed; with the top bit
/// set, in 256ths.
pub(super) fn number(parameter: u16) -> f32 {
    let value = ((parameter << 1) as i16 >> 1) as f32;
    match parameter & 0x8000 {
        0 => value,
        _ => value / 256.0,
    }
}

/// A position marker (four parameters: half-tile flags, then 8-pixel
/// tiles), in pixels.
pub(super) fn mark(p: &[u16]) -> Place {
    let half = |flag: u16| if flag >= 2 { 4.0 } else { 0.0 };
    Place::At(p[2] as f32 * 8.0 + half(p[0]), p[3] as f32 * 8.0 + half(p[1]))
}

/// The `Move...` and `Slide...` operations: `(Move|Slide)[23]Position`
/// then `Mark`, `Offset`, `Lives` or nothing (pixels); a speed, then one
/// place or, for those taking any number of parameters, several.
pub(super) fn movement(name: &str, p: &[u16]) -> Option<Act> {
    let (slide, rest) = match (name.strip_prefix("Move"), name.strip_prefix("Slide")) {
        (Some(rest), _) => (false, rest),
        (_, Some(rest)) => (true, rest),
        _ => return None,
    };
    let kind = rest.trim_start_matches(['2', '3']).strip_prefix("Position")?;
    let group = match kind {
        "Mark" => 4,
        "Offset" | "" => 2,
        "Lives" => 1,
        _ => return None,
    };
    // The speed comes first when a parameter is left over.
    let (speed, places) = match p.len() % group == 1 || group == 1 {
        true => (number(*p.first()?), &p[1..]),
        false => (1.0, p),
    };
    let path = places.chunks_exact(group).map(|place| match kind {
        "Mark" => mark(place),
        "Offset" => Place::By(number(place[0]), number(place[1])),
        "Lives" => Place::Actor(place[0]),
        _ => Place::At(number(place[0]), number(place[1])),
    });
    Some(Act::Move { speed: speed.max(0.25), path: path.collect(), slide })
}

/// The comparison operators of `BranchValue`, `CaseValue`...
pub(super) fn compare(left: i32, operator: u16, right: i32) -> bool {
    match operator {
        0 => false,
        1 => true,
        2 => left == right,
        3 => left > right,
        4 => left < right,
        5 => left >= right,
        6 => left <= right,
        7 => left != right,
        8 => left & right != 0,
        9 => left ^ right != 0,
        10 => left >> right & 1 != 0,
        _ => false,
    }
}

/// The assignment operators of `flag_CalcValue`...
pub(super) fn calculate(current: i32, operator: u16, operand: i32) -> i32 {
    match operator {
        0 => operand,
        1 => current - operand,
        2 => current + operand,
        3 => current * operand,
        4 if operand != 0 => current / operand,
        _ => current,
    }
}
